/*
 * Copyright (c) 2019 Web3 Technologies Foundation
 *
 * This file is part of Polkadot Host Test Suite
 *
 * Polkadot Host Test Suite is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Polkadot Host Tests is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Foobar.  If not, see <https://www.gnu.org/licenses/>.
 */

#include "helpers.hpp"

#include <fstream>

#include <crypto/pbkdf2/impl/pbkdf2_provider_impl.hpp>
#include <crypto/random_generator/boost_generator.hpp>
#include <crypto/ed25519/ed25519_provider_impl.hpp>
#include <crypto/sr25519/sr25519_provider_impl.hpp>
#include <crypto/secp256k1/secp256k1_provider_impl.hpp>
#include <crypto/hasher/hasher_impl.hpp>
#include <crypto/crypto_store/crypto_store_impl.hpp>
#include <crypto/bip39/impl/bip39_provider_impl.hpp>

#include <storage/in_memory/in_memory_storage.hpp>
#include <storage/trie/polkadot_trie/polkadot_trie_factory_impl.hpp>
#include <storage/trie/serialization/polkadot_codec.hpp>
#include <storage/trie/serialization/trie_serializer_impl.hpp>
#include <storage/trie/impl/trie_storage_backend_impl.hpp>
#include <storage/trie/impl/trie_storage_impl.hpp>
#include <storage/changes_trie/impl/storage_changes_tracker_impl.hpp>

#include <runtime/common/trie_storage_provider_impl.hpp>
#include <runtime/binaryen/runtime_manager.hpp>
#include <runtime/binaryen/module/wasm_module_factory_impl.hpp>
#include <runtime/binaryen/runtime_api/core_factory_impl.hpp>

#include <extensions/impl/extension_factory_impl.hpp>


using kagome::api::Session;

using kagome::primitives::BlockHash;

using kagome::blockchain::BlockHeaderRepository;

using kagome::crypto::Bip39ProviderImpl;
using kagome::crypto::BoostRandomGenerator;
using kagome::crypto::CryptoStoreImpl;
using kagome::crypto::Ed25519Suite;
using kagome::crypto::Sr25519Suite;
using kagome::crypto::Ed25519ProviderImpl;
using kagome::crypto::Sr25519ProviderImpl;
using kagome::crypto::KeyFileStorage;
using kagome::crypto::HasherImpl;
using kagome::crypto::Pbkdf2ProviderImpl;
using kagome::crypto::Secp256k1ProviderImpl;

using kagome::extensions::ExtensionFactoryImpl;

using kagome::runtime::TrieStorageProviderImpl;
using kagome::runtime::WasmProvider;

using kagome::runtime::binaryen::CoreFactoryImpl;
using kagome::runtime::binaryen::RuntimeManager;
using kagome::runtime::binaryen::WasmModuleFactoryImpl;

using kagome::storage::InMemoryStorage;
using kagome::storage::changes_trie::StorageChangesTrackerImpl;
using kagome::storage::trie::PolkadotCodec;
using kagome::storage::trie::PolkadotTrieFactoryImpl;
using kagome::storage::trie::TrieSerializerImpl;
using kagome::storage::trie::TrieStorageImpl;
using kagome::storage::trie::TrieStorageBackendImpl;

using kagome::subscription::SubscriptionEngine;

namespace helpers {

  // Path to wasm adapter shim runtime
  const char* WASM_ADAPTER_RUNTIME_PATH = "bin/hostapi_runtime.compact.wasm";

  // Simple wasm provider to provide wasm adapter runtime shim to kagome
  class WasmAdapterProvider : public WasmProvider {
  public:
    WasmAdapterProvider() {
      // Open file and determine size (ios::ate jumps to end on open)
      std::ifstream file(WASM_ADAPTER_RUNTIME_PATH, std::ios::binary | std::ios::ate);
      if (!file) {
        throw std::invalid_argument("Wasm adapter runtime not found!");
      }
      int size = file.tellg();
      file.seekg(0, std::ios::beg);

      // Load code into buffer
      code_.resize(size);
      file.read(reinterpret_cast<char *>(code_.data()), size);
    }

    const Buffer &getStateCode() const override {
      return code_;
    }

  private:
    Buffer code_;
  };


  RuntimeEnvironment::RuntimeEnvironment() {
    // Load wasm adapter shim
    auto wasm_provider = std::make_shared<WasmAdapterProvider>();

    // Build storage provider
    auto backend = std::make_shared<TrieStorageBackendImpl>(
      std::make_shared<InMemoryStorage>(), Buffer{}
    );

    auto trie_factory = std::make_shared<PolkadotTrieFactoryImpl>();
    auto codec = std::make_shared<PolkadotCodec>();
    auto serializer = std::make_shared<TrieSerializerImpl>(
      trie_factory, codec, backend
    );

    auto trie_db = TrieStorageImpl::createEmpty(
      trie_factory, codec, serializer, boost::none
    ).value();

    auto storage_provider = std::make_shared<TrieStorageProviderImpl>(
      std::move(trie_db)
    );

    // Build change tracker
    using SessionPtr = std::shared_ptr<Session>;
    using SubscriptionEngineType =
      SubscriptionEngine<Buffer, SessionPtr, Buffer, BlockHash>;

    auto sub_engine = std::make_shared<SubscriptionEngineType>();
    auto tracker = std::make_shared<StorageChangesTrackerImpl>(
      trie_factory, codec, sub_engine
    );

    // Build crypto providers
    auto pbkdf2_provider = std::make_shared<Pbkdf2ProviderImpl>();
    auto random_generator = std::make_shared<BoostRandomGenerator>();
    auto ed25519_provider = std::make_shared<Ed25519ProviderImpl>(random_generator);
    auto sr25519_provider = std::make_shared<Sr25519ProviderImpl>(random_generator);
    auto secp256k1_provider = std::make_shared<Secp256k1ProviderImpl>();
    auto hasher = std::make_shared<HasherImpl>();
    auto bip39_provider = std::make_shared<Bip39ProviderImpl>(pbkdf2_provider);

    auto keystore_path = boost::filesystem::temp_directory_path() / "kagome-adapter-host-api";
    auto crypto_store = std::make_shared<CryptoStoreImpl>(
      std::make_shared<Ed25519Suite>(ed25519_provider),
      std::make_shared<Sr25519Suite>(sr25519_provider),
      bip39_provider,
      KeyFileStorage::createAt(keystore_path).value()
    );

    repo_ = std::make_shared<KeyValueBlockHeaderRepository>(
      std::make_shared<InMemoryStorage>(), std::make_shared<HasherImpl>()
    );

    // Static factory method
    auto factory_method = [this, tracker](std::shared_ptr<WasmProvider> wasm_provider) {
      CoreFactoryImpl factory(
        runtime_manager_,
        tracker,
        repo_
      );
      return factory.createWithCode(std::move(wasm_provider));
    };

    // Assemble extension factory
    auto extension_factory = std::make_shared<ExtensionFactoryImpl>(
      tracker,
      sr25519_provider,
      ed25519_provider,
      secp256k1_provider,
      hasher,
      crypto_store,
      bip39_provider,
      factory_method
    );

    auto module_factory = std::make_shared<WasmModuleFactoryImpl>();

    runtime_manager_ = std::make_shared<RuntimeManager>(
      extension_factory,
      module_factory,
      storage_provider,
      hasher
    );

    runtime_ = std::make_shared<RawRuntimeApi>(wasm_provider, runtime_manager_);

    // Set expected defaults in storage
    auto eight_64bit = std::string("\x08\0\0\0\0\0\0\0", 8);

    execute<void>("rtm_ext_storage_set_version_1", ":code", "");
    execute<void>("rtm_ext_storage_set_version_1", ":heappages", eight_64bit);
  }

  std::shared_ptr<kagome::runtime::binaryen::RuntimeManager> RuntimeEnvironment::getRuntimeManager() {
    return runtime_manager_;
  }

} // namespace helper
