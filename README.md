# AEGIS-BIO — Kutup teması

Rust + egui tabanlı astronot ekranı ve görev kontrol uygulaması. Seçilen açık renkli **Kutup** teması doğrudan Rust arayüz koduna uygulanmıştır.

## Çalıştırma

Gereksinimler: edition 2024 destekleyen güncel Rust araç zinciri, Cargo ve Windows için Visual Studio C++ Build Tools / Windows SDK. İlk derleme bağımlılıkları internetten indirir. Komutları bu README'nin bulunduğu klasörde çalıştırın.

Astronot ekranı:

```powershell
cargo run --locked -p biomon-mobile --bin biomon-desktop-runner
```

Görev kontrol paneli (ayrı terminalde):

```powershell
cargo run --locked -p hub-desktop --bin hub-desktop
```

Derleme ve mevcut testler:

```powershell
cargo check --locked --workspace --all-targets
cargo test --locked --workspace
```

Release uygulamaları:

```powershell
cargo build --locked --release --workspace --bins
```

Üretilen dosyalar: `target/release/biomon-desktop-runner.exe` ve `target/release/hub-desktop.exe`.

## Tema

- Kırık beyaz yüzey, koyu yeşil metin, petrol yeşili vurgu.
- Astronot ana ekranındaki dört panel, merkez düğmesi, pusula ve giysi çizimi korunur.
- Ana ekran, mod düğmeleri, temel detay etiketleri ve görev kontrol gezinmesi Türkçeleştirilmiştir. Bu sürüm tam bir yerelleştirme değildir; bazı teknik detaylar ve veri kaynaklarından gelen metinler İngilizce kalır.
- Giriş/dikkat durumları amber, kritik uyarılar kırmızıdır.

Tema kodu: `biomon-mobile/src/ui_components.rs` ve `hub-desktop/src/ui_components.rs` içindeki `palette` modülleri. Değişikliklerin kapsamı ve doğrulama sınırı için `KUTUP_TEMASI.md` dosyasına bakın.

## Kaynak yapısı

Etkin Cargo workspace üyeleri `biomon-mobile` ve `hub-desktop` paketleridir. ZIP'ten gelen kök `src/`, `examples/` ve Python `hub/` dosyaları korunmuştur; eski kök Rust örnekleri bu workspace'in derleme hedefleri değildir. Üstteki komutlar etkin paketleri çalıştırır.

Bu paketin hazır `.exe` dosyası yoktur. Hazırlanan bilgisayarda Cargo/rustc bulunmadığından derleme, test çalıştırma ve gerçek egui ekran görüntüsü doğrulanamamıştır.
