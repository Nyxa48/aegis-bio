# AEGIS-BIO — NASA Space Apps 2026

**Challenge:** [Create Health Monitoring Software for Astronauts on Space Missions](https://www.spaceappschallenge.org/2026/challenges/health-monitoring-software/)

AEGIS-BIO, astronot bilek biomonitörü (`aegis_bio_demo`), görev kontrol masaüstü terminali (`hub_terminal`) ve paylaşılan UI tasarım bileşenlerinden (`examples/ui_components.rs`) oluşan Rust/`egui` tabanlı bir sağlık ve görev izleme arayüzüdür.

## Çalıştırma Komutları

Depo kök dizinine geçin:

```powershell
cd C:\Users\Emir\Documents\GitHub\aegis-bio
```

### 1. Biomon (Astronot Bilek / Tablet Ekranı)
```powershell
cargo run --bin aegis_bio_demo
# veya varsayılan binary olduğu için:
cargo run
```

### 2. Hub Desktop (Görev Kontrol Masaüstü Terminali)
```powershell
cargo run --bin hub_terminal
```

### 3. Sadece UI Bileşenleri Vitrini (Tasarım Ekibi İçin)
Tüm UI paleti, kartlar, butonlar, slider, pusula ve giysi şematiğini bağımsız tek dosyada görmek için:
```powershell
cargo run --example ui_components
```
> Kaynak dosya: `examples/ui_components.rs`

### 4. Test & Derleme Kontrolü
```powershell
cargo test
cargo build --bins
```

## Ekip (4 Kişi)
- **Ekip Lideri / Sistem Mimarı & Fizyolojik Motor:** Emir (`Nyxa48`)
- **Veri & Simülasyon Mühendisi:** *(Eklenecek)*
- **Görev Kontrol Frontend & Görselleştirme:** *(Eklenecek)*
- **Astronot Beslenme UI, NASA Referansları & Medya:** *(Eklenecek)*
