{
  "targets": [
    {
      "target_name": "pdf_oxide",
      "sources": ["binding.cc"],
      "include_dirs": [
        "<!@(node -p \"require('node-addon-api').include\")"
      ],
      "dependencies": [
        "<!(node -p \"require('node-addon-api').gyp\")"
      ],
      "cflags": ["-Wall", "-Wextra"],
      "cflags_cc": ["-fexceptions"],
      "xcode_settings": {
        "GCC_ENABLE_CPP_EXCEPTIONS": "YES",
        "CLANG_CXX_LIBRARY": "libc++"
      },
      "msvs_settings": {
        "VCCLCompilerTool": {
          "ExceptionHandling": 1
        }
      },
      "conditions": [
        ["OS==\"linux\"", {
          "libraries": [
            "<(module_root_dir)/../lib/libpdf_oxide.a",
            "-lm",
            "-lpthread",
            "-ldl",
            "-lrt",
            "-lgcc_s",
            "-lutil"
          ]
        }],
        ["OS==\"mac\"", {
          "libraries": [
            "<(module_root_dir)/../lib/libpdf_oxide.a",
            "-framework", "CoreFoundation",
            "-framework", "Security",
            "-framework", "SystemConfiguration",
            "-liconv",
            "-lresolv"
          ]
        }],
        ["OS==\"win\"", {
          "libraries": [
            "<(module_root_dir)/../lib/pdf_oxide.lib",
            "ws2_32.lib",
            "userenv.lib",
            "bcrypt.lib",
            "advapi32.lib",
            "crypt32.lib",
            "ntdll.lib",
            "kernel32.lib",
            "ole32.lib",
            "shell32.lib",
            "synchronization.lib"
          ]
        }]
      ]
    }
  ]
}
