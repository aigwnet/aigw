ARCH=$1
PACKAGE=$2

CC=${ARCH}-linux-musl-gcc cargo build --package $PACKAGE --release \
  --target=${ARCH}-unknown-linux-musl \
  --config=target.${ARCH}-unknown-linux-musl.linker=\"${ARCH}-linux-musl-gcc\"