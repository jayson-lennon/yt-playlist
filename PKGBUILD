# Maintainer: Jayson Lennon <jayson@jaysonlennon.dev>

pkgname=shownotes
pkgver=0.5.1
pkgrel=1
pkgdesc='manage shownotes for podcast/video'
url=''
license=(GPL-3.0-only)
makedepends=('cargo')
depends=('mpv' 'skim' 'xdg-utils')
arch=('i686' 'x86_64' 'armv6h' 'armv7h')

# No source array needed - we reference files directly from $startdir
# This avoids conflicts with the project's src/ directory

# Dedicated build directory outside of project's src/ folder
_builddir="$startdir/.build/$pkgname-$pkgver"

prepare() {
    # Create dedicated build directory
    rm -rf "$_builddir"
    mkdir -p "$_builddir"

    # Copy Rust source files to build directory
    cp -r "$startdir/crates" "$_builddir/"
    cp -r "$startdir/tests" "$_builddir/"
    cp "$startdir/Cargo.toml" "$_builddir/"
    cp "$startdir/Cargo.lock" "$_builddir/"

    # Fetch dependencies in build directory
    cd "$_builddir"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$_builddir"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR="$_builddir/target"
    # needed if using sqlx+sqlite
    CFLAGS+=" -ffat-lto-objects" cargo build --frozen --release --all-features
}

check() {
    cd "$_builddir"
    export RUSTUP_TOOLCHAIN=stable
}

package() {
    local _buildtarget="$_builddir/target/release"

    # Install binaries
    install -Dm0755 -t "$pkgdir/usr/bin/" "$_buildtarget/shownotes"
}
