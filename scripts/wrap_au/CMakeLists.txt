cmake_minimum_required(VERSION 3.21)
if (NOT DEFINED CMAKE_OSX_DEPLOYMENT_TARGET)
  set(CMAKE_OSX_DEPLOYMENT_TARGET 10.13 CACHE STRING "Minimum macOS version")
endif()
project(daw_agent_au LANGUAGES C CXX OBJC OBJCXX)

# clap-wrapper only sets CMAKE_CXX_STANDARD when it is the top-level project.
# As a subdirectory it inherits ours; if unset, AppleClang compiles as C++98
# and AudioUnitSDK / .mm sources fail with -Wc++11-extensions.
set(CMAKE_CXX_STANDARD 17 CACHE STRING "C++ standard for clap-wrapper + AudioUnitSDK" FORCE)
set(CMAKE_CXX_STANDARD_REQUIRED ON CACHE BOOL "" FORCE)
set(CMAKE_OBJCXX_STANDARD 17 CACHE STRING "ObjC++ standard for clap-wrapper .mm" FORCE)
set(CMAKE_OBJCXX_STANDARD_REQUIRED ON CACHE BOOL "" FORCE)
set(CLAP_WRAPPER_CXX_STANDARD 17 CACHE STRING "" FORCE)

message(STATUS "daw_agent_au: CMAKE_CXX_STANDARD=${CMAKE_CXX_STANDARD} CMAKE_OBJCXX_STANDARD=${CMAKE_OBJCXX_STANDARD}")

if (NOT APPLE)
  message(FATAL_ERROR "AUv2 wrapping requires macOS")
endif()
if (NOT DEFINED CLAP_WRAPPER_SRC)
  message(FATAL_ERROR "CLAP_WRAPPER_SRC must point at a clap-wrapper checkout")
endif()
if (NOT DEFINED DAW_CLAP_PATH)
  message(FATAL_ERROR "DAW_CLAP_PATH must point at DAWAgent.clap")
endif()

set(CLAP_WRAPPER_DOWNLOAD_DEPENDENCIES ON CACHE BOOL "" FORCE)
add_subdirectory("${CLAP_WRAPPER_SRC}" clap-wrapper)

add_library(DAWAgent_as_auv2 MODULE)
target_add_auv2_wrapper(
  TARGET DAWAgent_as_auv2
  OUTPUT_NAME "DAWAgent"
  BUNDLE_IDENTIFIER "com.dawmcp.plugin"
  BUNDLE_VERSION "0.1.0"
  INSTRUMENT_TYPE "aufx"
  MANUFACTURER_NAME "DAW Plugin MCP"
  MANUFACTURER_CODE "DwMc"
  SUBTYPE_CODE "Agnt"
  MACOS_EMBEDDED_CLAP_LOCATION "${DAW_CLAP_PATH}"
)
