targets="$(bazel query "kind('rust_.*', //...)")"
ibazel build \
    --bazelrc=$(pwd)/.vscode/rust/ibazel.bazelrc \
    $targets
