# Install script for directory: /Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include

# Set the install prefix
if(NOT DEFINED CMAKE_INSTALL_PREFIX)
  set(CMAKE_INSTALL_PREFIX "/Users/piyushdaiya/Documents/projects/waveshare-epd397-rust-app/target/xtensa-esp32s3-espidf/debug/build/esp-idf-sys-fd3c89afb5648272/out")
endif()
string(REGEX REPLACE "/$" "" CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")

# Set the install configuration name.
if(NOT DEFINED CMAKE_INSTALL_CONFIG_NAME)
  if(BUILD_TYPE)
    string(REGEX REPLACE "^[^A-Za-z0-9_]+" ""
           CMAKE_INSTALL_CONFIG_NAME "${BUILD_TYPE}")
  else()
    set(CMAKE_INSTALL_CONFIG_NAME "")
  endif()
  message(STATUS "Install configuration: \"${CMAKE_INSTALL_CONFIG_NAME}\"")
endif()

# Set the component getting installed.
if(NOT CMAKE_INSTALL_COMPONENT)
  if(COMPONENT)
    message(STATUS "Install component: \"${COMPONENT}\"")
    set(CMAKE_INSTALL_COMPONENT "${COMPONENT}")
  else()
    set(CMAKE_INSTALL_COMPONENT)
  endif()
endif()

# Is this installation the result of a crosscompile?
if(NOT DEFINED CMAKE_CROSSCOMPILING)
  set(CMAKE_CROSSCOMPILING "TRUE")
endif()

# Set path to fallback-tool for dependency-resolution.
if(NOT DEFINED CMAKE_OBJDUMP)
  set(CMAKE_OBJDUMP "/Users/piyushdaiya/.espressif/tools/xtensa-esp-elf/esp-14.2.0_20250730/xtensa-esp-elf/bin/xtensa-esp32s3-elf-objdump")
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include/mbedtls" TYPE FILE PERMISSIONS OWNER_READ OWNER_WRITE GROUP_READ WORLD_READ FILES
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/aes.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/aria.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/asn1.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/asn1write.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/base64.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/bignum.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/block_cipher.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/build_info.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/camellia.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ccm.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/chacha20.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/chachapoly.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/check_config.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/cipher.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/cmac.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/compat-2.x.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_adjust_legacy_crypto.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_adjust_legacy_from_psa.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_adjust_psa_from_legacy.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_adjust_psa_superset_legacy.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_adjust_ssl.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_adjust_x509.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/config_psa.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/constant_time.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ctr_drbg.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/debug.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/des.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/dhm.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ecdh.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ecdsa.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ecjpake.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ecp.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/entropy.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/error.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/gcm.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/hkdf.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/hmac_drbg.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/lms.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/mbedtls_config.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/md.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/md5.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/memory_buffer_alloc.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/net_sockets.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/nist_kw.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/oid.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/pem.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/pk.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/pkcs12.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/pkcs5.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/pkcs7.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/platform.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/platform_time.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/platform_util.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/poly1305.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/private_access.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/psa_util.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ripemd160.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/rsa.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/sha1.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/sha256.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/sha3.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/sha512.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ssl.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ssl_cache.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ssl_ciphersuites.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ssl_cookie.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/ssl_ticket.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/threading.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/timing.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/version.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/x509.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/x509_crl.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/x509_crt.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/mbedtls/x509_csr.h"
    )
endif()

if(CMAKE_INSTALL_COMPONENT STREQUAL "Unspecified" OR NOT CMAKE_INSTALL_COMPONENT)
  file(INSTALL DESTINATION "${CMAKE_INSTALL_PREFIX}/include/psa" TYPE FILE PERMISSIONS OWNER_READ OWNER_WRITE GROUP_READ WORLD_READ FILES
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/build_info.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_adjust_auto_enabled.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_adjust_config_dependencies.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_adjust_config_key_pair_types.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_adjust_config_synonyms.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_builtin_composites.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_builtin_key_derivation.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_builtin_primitives.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_compat.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_config.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_driver_common.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_driver_contexts_composites.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_driver_contexts_key_derivation.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_driver_contexts_primitives.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_extra.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_legacy.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_platform.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_se_driver.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_sizes.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_struct.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_types.h"
    "/Users/piyushdaiya/.espressif/esp-idf/v5.4.3/components/mbedtls/mbedtls/include/psa/crypto_values.h"
    )
endif()

