load("@rules_foreign_cc//foreign_cc:defs.bzl", "cmake")

package(default_visibility = ["//visibility:public"])

cmake(
    name = "katago_cmake",
    cache_entries = {
        "USE_BACKEND": "CUDA",
        "BUILD_DISTRIBUTED": "0",
        "NO_GIT_REVISION": "1",
    },
    env = {
        "CMAKE_BUILD_TYPE": "Release",
        "CMAKE_BUILD_PARALLEL_LEVEL": "24",
    },
    working_directory = "cpp",
    lib_source = "@katago//:all",
    out_binaries = ["katago"],
)

genrule(
    name = "katago",
    srcs = [":katago_cmake"],
    outs = ["katago_binary"],
    cmd = """
    for f in $(SRCS); do
        case "$$f" in
            */bin/katago)
                cp "$$f" "$@"
                chmod +x "$@"
                exit 0
                ;;
        esac
    done
    echo "ERROR: katago binary not found in sources: $(SRCS)" >&2
    exit 1
    """,
    visibility = ["//visibility:public"],
    executable = True,
)
