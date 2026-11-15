# AEGIS-BIO — NASA Space Apps 2026

> **[English](#english-documentation)** · **[Türkçe](#türkçe-dokümantasyon)**

**Challenge:** [Create Health Monitoring Software for Astronauts on Space Missions](https://www.spaceappschallenge.org/2026/challenges/health-monitoring-software/)

---

## English Documentation

**AEGIS-BIO** is an offline-first astronaut biometric monitoring and mission control software suite written in Rust (`eframe`/`egui`, `rusqlite`, `tokio`). Built for deep-space and orbital exploration scenarios—where Earth communication delays and habitat link blackouts are routine—the system combines an autonomous wrist/tablet **Astronaut Biomonitor (`biomon-mobile`)** with a **Mission Control Desktop Hub (`hub-desktop`)** backed by NASA open datasets.

### System Architecture

AEGIS-BIO runs as a two-node Rust Cargo workspace communicating over LAN TCP (newline-delimited JSON) while maintaining local resilience through embedded SQLite databases in **WAL** (Write-Ahead Logging) mode:

1. **Astronaut Biomonitor (`biomon-mobile`)** — Cross-platform HUD targeting both Android tablets/wearables (`space.aegisbio.biomon`, `aarch64-linux-android`, `sensorLandscape`, `minSdk 26` / `targetSdk 34`) and desktop simulation (`biomon-desktop-runner`).
   - **4-Quadrant Main HUD:** `HEALTH & SUIT`, `HUB & NAV`, `COMMS`, and `HAZARD & ENV`, with responsive scaling across phone, tablet, and desktop aspect ratios.
   - **Dual-Scope Glove-Friendly UX:**
     - **VIEW Mode:** Full-area telemetry cockpit optimized for zero-distraction readability; tap anywhere to return from detail overlays.
     - **INPUT Mode:** Large touch targets for logging ISS meals (with `25%` / `50%` / `100%` consumption ratios), hydration intake, exercise, simulation scenarios, blackout toggling, and local alert acknowledgements.
   - **Offline Autonomy (`aegis_biomon.db`):** Queues 1 Hz telemetry frames, crew inputs, 60-second actigraphy sleep epochs, and daily SANS/osteoporosis states locally, alongside deterministic offline countermeasure protocols (`CARDIO`, `DECOMP`, `HYDRATION`, `HYPOXIA`, `RADIATION`).

2. **Mission Control Hub (`hub-desktop`)** — Habitat / Flight Surgeon desktop console (`hub-desktop`) listening for incoming telemetry packets on `0.0.0.0:7878` and persisting mission records to `aegis_hub.db`.
   - **CREW VITALS MONITOR:** Real-time physiological overview, shock index, orthostatic risk, SANS risk, and LAN sync state across crew members.
   - **PHASE & STRESS CONTROL:** Mission phase switching (`LAUNCH`, `ORBIT`, `EVA`, `REENTRY`) and stress scenario injection (`TACHYCARDIA SPIKE`, `HYPOXIA DROP`, `SOLAR PARTICLE RADIATION SPIKE`, `CABIN CO2 SCRUBBING FAILURE`, `SUIT DECOMPRESSION / N2 BREACH`, `HUB RANGE EXCURSION`).
   - **MEDICAL & INCIDENT LOG:** Flight surgeon incident tracking, clinical notes, crew input audit log, and NASA OSDR research/reference inspection.
   - **FOOD CATALOG & SYNC:** ISS space food catalog (`iss_food_catalog`), daily nutritional intake & SANS/osteoporosis summaries, LAN server status, and dataset provenance ledger (`dataset_provenance`).

### Biomedical Models & NASA Open Datasets

- **Physiological & Risk Engine (`calc.rs` & `sim.rs`):**
  - **Keytel et al. (2005)** heart-rate-based energy expenditure (`kcal/s`) and thermal evaporative sweat loss estimation.
  - **NASA-STD-3001 Dynamic Heart-Rate Corridors:** Phase-aware safety bounds (`ORBIT`: `45–100 BPM`, `LAUNCH`/`REENTRY`: `70–150 BPM`, `EVA`: `75–160 BPM`).
  - **Haldane Decompression Model:** Blood/tissue N₂ ratio monitoring (`HALDANE_HALF_TIME_MIN = 360.0`) and EVA suit pressure breach detection.
  - **Hypoxia, Shock & SANS / Osteoporosis Modeling:** Continuous evaluation of SpO₂, tissue O₂, Shock Index (`HR / SBP`), cumulative dietary sodium + cabin CO₂ (SANS risk), and exercise load + calcium + vitamin D3 (osteoporosis protection).
  - **Earth–Mars Ephemeris & Light Delay:** One-way signal propagation time (`8.31675 min/AU`) derived from JPL Horizons ephemeris data.
- **NASA Reference Datasets (`scripts/build_nasa_seed.py` -> `hub-desktop/data/nasa_seed.sql`):**
  - **JPL Horizons:** Earth–Mars orbital distance and one-way light-time ephemeris (`mission_ephemeris`).
  - **NASA OSDR RadLab:** Historical radiation absorbed dose rate time series (`reference_radiation_series`).
  - **NASA OSDR EDA (RR-1):** ISS vs. ground control CO₂, temperature, and humidity reference telemetry (`reference_eda_environment`).
  - **NASA OSDR Studies (`OSD-569`, `OSD-575`, `OSD-656`):** Transformed CBC, CMP, cardiovascular, and serum/urine immune biomarker panels (`research_lab_observations` & `hub_lab_reports`).

### Getting Started

#### Prerequisites
- **Rust toolchain** (edition `2024`; verified with `rustc 1.100.0` / `cargo 1.100.0`)
- *(Optional for Android APK builds)* Android NDK + `cargo-apk` targeting `aarch64-linux-android`
- *(Optional for dataset ETL regeneration / legacy FastAPI prototype)* Python `>= 3.11` & `uv`

#### Running from Source

1. **Start the Mission Control Hub** (starts the TCP listener on `0.0.0.0:7878` and seeds `aegis_hub.db`):
   ```powershell
   cargo run --bin hub-desktop
   ```

2. **Start the Astronaut Biomonitor (Desktop Simulation)** (connects to `127.0.0.1:7878` and seeds `aegis_biomon.db`):
   ```powershell
   cargo run --bin biomon-desktop-runner
   ```

3. **Run Tests & Workspace Verification:**
   ```powershell
   cargo test --workspace
   cargo check --workspace --bins
   ```

> **Build Cache Tip:** `Cargo.toml` configures compact workspace profiles (`debug = "line-tables-only"`, external dependencies at `debug = 0`, and release `strip = "debuginfo"`) to keep `target/` build caches lean across developer machines.

#### Prebuilt Releases
Prebuilt Windows (`hub-desktop.exe`) and Android (`aegis-biomon.apk`) binaries are available on the [GitHub Releases](https://github.com/Nyxa48/aegis-bio/releases) page.

### Repository Structure

```text
aegis-bio/
├── Cargo.toml                  # Cargo workspace configuration & compact dev/release profiles
├── biomon-mobile/              # Astronaut Biomonitor HUD crate (Android cdylib + desktop runner)
│   ├── sql/init.sql            # Biomon SQLite WAL schema & derived nutrition/CO2 views
│   └── src/                    # app, calc, db, icons, net, sim, ui, ui_components
├── hub-desktop/                # Mission Control Desktop Hub crate
│   ├── data/nasa_seed.sql      # Normalized NASA Horizons, RadLab, EDA & OSDR SQLite seed
│   ├── sql/init.sql            # Hub SQLite WAL schema & dataset provenance tables
│   └── src/                    # app, calc, db, icons, net, ui, ui_components
├── scripts/build_nasa_seed.py  # Python ETL script generating nasa_seed.sql from NASA datasets
├── data/                       # Sample dataset & state snapshots
├── examples/ & src/            # Early prototype UI showcase & single-crate demo files
└── hub/ & tests/               # Earlier FastAPI + SQLite cockpit prototype & pytest suite
```

### Team (5 Members)

- **Emir Samed Y.** — [@Nyxa48](https://github.com/Nyxa48) · *Project Lead, Physiological Engine, Database & LAN Sync*
- **Ege Canbaz** — [@Glarman](https://github.com/Glarman) · *UI Architecture & Extended Interface Engineering*
- **Emin Sancaklı** — [@eminsancakli1](https://github.com/eminsancakli1) · *Dataset Research & Data Integration Analysis*
- **Emre Düzcan** —[@Lykablyat](https://github.com/Lykablyat) · *Systems Architecture, Workspace Modularization & Cross-Platform Application Scaffold*
- **Harun Karakoyun** —[@harunkarakoyuns](https://github.com/harunkarakoyuns) · *Research & General Project Support*

---

## Türkçe Dokümantasyon

**AEGIS-BIO**, **NASA Space Apps 2026** kapsamındaki *Create Health Monitoring Software for Astronauts on Space Missions* görevi için Rust (`eframe`/`egui`, `rusqlite`, `tokio`) ile geliştirilmiş, **çevrimdışı öncelikli (offline-first)** bir astronot biyometrik izleme ve görev kontrol yazılım sistemidir. Derin uzay ve yörünge görevlerinde iletişim gecikmeleri ile bağlantı kesintilerinin (blackout) olağan olduğu senaryolar düşünülerek tasarlanmıştır.

### Sistem Mimarisi

AEGIS-BIO, **WAL** (Write-Ahead Logging) modunda çalışan gömülü SQLite veritabanları ve yerel ağ TCP (NDJSON) senkronizasyonu ile haberleşen iki ana Rust crate'inden oluşur:

1. **Astronot Biyomonitörü (`biomon-mobile`)** — Android tablet/bileklik (`space.aegisbio.biomon`, `aarch64-linux-android`, `sensorLandscape`, `minSdk 26` / `targetSdk 34`) ve masaüstü simülasyonu (`biomon-desktop-runner`) için çapraz platform HUD uygulaması.
   - **4 Kadranlı Ana HUD:** Telefon, tablet ve masaüstü ekran oranlarına uyumlu duyarlı (responsive) düzeniyle `HEALTH & SUIT`, `HUB & NAV`, `COMMS` ve `HAZARD & ENV` panelleri.
   - **Çift Kapsamlı ve Eldiven Dostu Arayüz:**
     - **VIEW Modu:** Astronotun yalnızca canlı verileri dikkati dağılmadan görmesi için tam alan telemetri görünümü; detay ekranlarında herhangi bir yere dokunarak geri dönüş.
     - **INPUT Modu:** ISS öğün kaydı (`%25` / `%50` / `%100` tüketim oranlarıyla), sıvı alımı, egzersiz kaydı, simülasyon senaryosu seçimi, yapay bağlantı kesintisi (blackout) testi ve yerel alarm onaylama kontrolleri.
   - **Çevrimdışı Otonomi (`aegis_biomon.db`):** 1 Hz telemetri çerçevelerini, ekip girdilerini, 60 saniyelik uyku/aktigrafi pencerelerini, günlük SANS/osteoporoz durumlarını ve deterministik çevrimdışı müdahale protokollerini (`CARDIO`, `DECOMP`, `HYDRATION`, `HYPOXIA`, `RADIATION`) yerel olarak saklar.

2. **Görev Kontrol Merkezi (`hub-desktop`)** — `0.0.0.0:7878` adresi üzerinden astronot düğümlerinden gelen TCP paketlerini dinleyen ve görev kayıtlarını `aegis_hub.db` veritabanında tutan masaüstü kontrol terminali.
   - **CREW VITALS MONITOR:** Mürettebatın anlık hayati bulguları, şok indeksi, ortostatik risk, SANS riski ve LAN senkronizasyon durumu.
   - **PHASE & STRESS CONTROL:** Görev fazı yönetimi (`LAUNCH`, `ORBIT`, `EVA`, `REENTRY`) ve stres/anomali senaryosu enjeksiyonu (`TACHYCARDIA SPIKE`, `HYPOXIA DROP`, `SOLAR PARTICLE RADIATION SPIKE`, `CABIN CO2 SCRUBBING FAILURE`, `SUIT DECOMPRESSION / N2 BREACH`, `HUB RANGE EXCURSION`).
   - **MEDICAL & INCIDENT LOG:** Uçuş hekimi olay kayıtları, klinik notlar, astronot girdi geçmişi ve NASA OSDR referans laboratuvar verileri.
   - **FOOD CATALOG & SYNC:** ISS uzay gıda kataloğu (`iss_food_catalog`), günlük beslenme ve SANS/osteoporoz özetleri, TCP sunucu metrikleri ve veri kaynağı doğrulama kayıtları (`dataset_provenance`).

### Biyomedikal Modeller ve NASA Açık Veri Setleri

- **Fizyolojik Motor & Risk Hesaplamaları (`calc.rs` & `sim.rs`):**
  - **Keytel et al. (2005)** nabız temelli kalori harcaması (`kcal/sn`) ve deri sıcaklığına bağlı terleme sıvı kaybı hesabı.
  - **NASA-STD-3001 Faz Bazlı Nabız Koridorları:** Görev fazına göre dinamik güvenlik eşikleri (`ORBIT`: `45–100 BPM`, `LAUNCH`/`REENTRY`: `70–150 BPM`, `EVA`: `75–160 BPM`).
  - **Haldane Dekompresyon Modeli:** Azot oranı takibi (`HALDANE_HALF_TIME_MIN = 360.0`) ve EVA giysi basınç düşüşü algılama.
  - **Hipoksi, Şok İndeksi, SANS ve Osteoporoz Modelleri:** SpO₂, doku O₂, Şok İndeksi (`HR / SBP`), sodyum + kabin CO₂ kaynaklı SANS riski ve egzersiz + kalsiyum + D3 vitamini koruma skoru takibi.
  - **Dünya–Mars Işık Gecikmesi:** JPL Horizons efemeris verileriyle tek yönlü sinyal gecikmesi (`8.31675 dk/AU`) hesabı.
- **NASA Referans Veri Setleri (`scripts/build_nasa_seed.py` -> `hub-desktop/data/nasa_seed.sql`):**
  - **JPL Horizons:** Dünya–Mars mesafesi ve tek yönlü ışık süresi tablosu (`mission_ephemeris`).
  - **NASA OSDR RadLab:** Tarihsel radyasyon soğurulan doz hızı zaman serisi (`reference_radiation_series`).
  - **NASA OSDR EDA (RR-1):** ISS ve yer kontrol CO₂, sıcaklık ve nem karşılaştırma verileri (`reference_eda_environment`).
  - **NASA OSDR Çalışmaları (`OSD-569`, `OSD-575`, `OSD-656`):** CBC, CMP, kardiyovasküler ve serum/idrar immün biyobelirteç panelleri (`research_lab_observations` & `hub_lab_reports`).

### Kurulum ve Çalıştırma

1. **Görev Kontrol Merkezini (Hub) Başlatın** (`0.0.0.0:7878` TCP dinleyicisini başlatır ve `aegis_hub.db` dosyasını hazırlar):
   ```powershell
   cargo run --bin hub-desktop
   ```

2. **Astronot Biyomonitörünü Başlatın** (`127.0.0.1:7878` adresine bağlanır ve `aegis_biomon.db` dosyasını hazırlar):
   ```powershell
   cargo run --bin biomon-desktop-runner
   ```

3. **Testler ve Workspace Doğrulaması:**
   ```powershell
   cargo test --workspace
   cargo check --workspace --bins
   ```

> **Derleme Boyutu Notu:** [Cargo.toml](Cargo.toml) içinde tanımlı kompakt profiller (`debug = "line-tables-only"`, dış bağımlılıklar için `debug = 0`, release için `strip = "debuginfo"`) sayesinde geliştirme önbelleği (`target/`) diskte minimum yer kaplar. Hazır Android APK (`aegis-biomon.apk`) ve Windows EXE (`hub-desktop.exe`) paketlerine [GitHub Releases](https://github.com/Nyxa48/aegis-bio/releases) sayfasından ulaşabilirsiniz.

### Ekip (5 Kişi)

- **Emir Samed Y.** — [@Nyxa48](https://github.com/Nyxa48) · *Proje Lideri, Fizyolojik Motor, Veritabanı & LAN Senkronizasyonu*
- **Ege Canbaz** — [@Glarman](https://github.com/Glarman) · *Arayüz Mimarisi & Genişletilmiş Arayüz Geliştirme*
- **Emin Sancaklı** — [@eminsancakli1](https://github.com/eminsancakli1) · Veri Seti Araştırması & Veri Kullanım Analizi*
- **Emre Düzcan** —[@Lykablyat](https://github.com/Lykablyat) · *Sistem Mimarisi, Workspace Modülerizasyonu & Çapraz Platform Uygulama İskeleti*
- **Harun Karakoyun** —[@harunkarakoyuns](https://github.com/harunkarakoyuns) · *Araştırma & Genel Proje Desteği*
