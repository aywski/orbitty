# Maintainer: Arsenii Sahalianov <aywski@gmail.com>
pkgname=orbitty
pkgver=0.1.0
pkgrel=1
pkgdesc="Terminal idle screensaver with spinning planets"
arch=('x86_64' 'aarch64')
url="https://github.com/aywski/orbitty"
license=('MIT')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --locked
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/orbitty" "$pkgdir/usr/bin/orbitty"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
