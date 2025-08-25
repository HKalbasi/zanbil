#include <cstring>
#include <iostream>
#include <sstream>
#include <string>
#include <unistd.h>
#include <unordered_map>
#include <vector>

// Split helper
std::vector<std::string> split(const std::string &s, char delimiter) {
  std::vector<std::string> tokens;
  std::stringstream ss(s);
  std::string item;
  while (std::getline(ss, item, delimiter)) {
    tokens.push_back(item);
  }
  return tokens;
}

std::string rust_to_zig(const std::string &rust_target) {
  auto parts = split(rust_target, '-');
  if (parts.size() < 3)
    return rust_target; // not a standard triple

  std::string arch = parts[0];
  std::string vendor = parts[1];
  std::string os = parts[2];
  std::string abi = parts.size() > 3 ? parts[3] : "";

  // Map Rust OS/vendor to Zig OS
  std::unordered_map<std::string, std::string> os_map = {
      {"darwin", "macos"},
      {"windows", "windows"},
      {"linux", "linux"},
      {"none", "freestanding"},
      {"unknown", "freestanding"}};

  if (os_map.count(os)) {
    os = os_map[os];
  }

  // Special case: wasm
  if (arch.rfind("wasm32", 0) == 0) {
    if (os == "freestanding")
      return "wasm32-freestanding";
    if (os == "wasi")
      return "wasm32-wasi";
  }

  // Construct Zig triple
  std::string zig_target = arch + "-" + os;
  if (!abi.empty()) {
    zig_target += "-" + abi;
  }

  return zig_target;
}

int main(int argc, char *argv[]) {

  std::vector<char *> args;
  args.push_back((char *)"zig");

  std::string bin_name = argv[0];

  if (bin_name.find("++") != std::string::npos) {
    args.push_back((char *)"c++");
  } else {
    args.push_back((char *)"cc");
  }

  for (int i = 1; i < argc; i++) {
    std::string current = argv[i];

    if (current == "--target" && i + 1 < argc) {
      // Convert Rust target to Zig target
      std::string zig_target = rust_to_zig(argv[i + 1]);
      args.push_back((char *)"--target");
      args.push_back(strdup(zig_target.c_str())); // strdup keeps memory alive
      i++;                                        // skip the consumed value
    } else if (current.rfind("--target=", 0) == 0) {
      // Handle --target=<triple>
      std::string rust_target = current.substr(9);
      std::string zig_target = rust_to_zig(rust_target);
      std::string new_arg = "--target=" + zig_target;
      args.push_back(strdup(new_arg.c_str()));
    } else {
      args.push_back(argv[i]);
    }
  }

  for (const auto &arg : args) {
    std::cout << arg << " ";
  }

  std::cout << std::endl;

  args.push_back(nullptr);

  execvp("zig", args.data());

  return 1;
}