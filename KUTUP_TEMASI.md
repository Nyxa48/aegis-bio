# Kutup uygulama notları

Bu teslim, mevcut Rust + egui kaynak kodunun değiştirilmiş kopyasıdır. Orijinal masaüstü ZIP dosyası değiştirilmedi.

## Uygulanan renkler

- Ana yüzey: `#F7F8F3` (seçilen Kutup önizlemesindeki kırık beyaz).
- Dış zemin: `#E6E9E5`.
- Ana metin: `#173B3B`.
- Vurgu ve görüntüleme modu: `#187E78`.
- Panel çerçevesi: `#719A93`.
- Normal durum: `#1A7862`.
- Giriş/dikkat durumu: `#A65C16`.
- Kritik uyarı: `#CD0929`.

Her iki etkin Rust paketindeki renk paleti güncellendi. Hover, basılı durum, seçim, bağlantı ve izleme çubuğu renkleri açık temayla eşleştirildi. Astronot ana ekranındaki panel sınırları, merkez düğmenin konumu ve yarıçapı, pusula ve giysi çizim koordinatları korunmuştur. Merkezdeki Türkçe mod yazısının boyutu mevcut daireye sığacak şekilde ölçeklenir.

Ana ekran başlıkları: BİYOMETRİ, KONUM, BAĞLANTI, KABİN ORTAMI. Merkez düğme: KONTROL; modlar: GÖRÜNTÜLE / VERİ GİRİŞİ. Bazı detay ekranı ve görev kontrol etiketleri de Türkçeleştirildi.

## Değişmeyen işlevler

Hesaplamalar, tıbbi eşik değerleri, SQL/veri şeması, NASA veri dosyaları, ağ paketlerinin alanları ve görev fazlarının kodları değiştirilmedi. `ConnectionState::label()` yalnızca ekranda görülen metinleri döndürür; enum değerleri ve Serde temsili aynı kalır. Mevcut uygulamanın simülasyon ve veri davranışı korunur; görünüm değişikliği yeni bir canlı sensör bağlantısı eklemez.

## Doğrulama

- Paket, orijinal ZIP ile dosya bazında karşılaştırıldı; değişiklikler arayüz sunumu ve belgelendirmeyle sınırlıdır.
- Etkin Cargo hedefleri ve çalıştırma komutları manifestlerden kontrol edildi.
- Kaynak düzeyinde arayüz geometrisi, hesaplama/veri/ağ işleyişi ve bağımlılık dosyalarının korunması kontrol edildi.
- Bu bilgisayarda Rust/Cargo kurulu olmadığından `cargo check`, `cargo test` ve gerçek egui penceresi çalıştırılamadı. Derlendiği veya görsel çalışma testinden geçtiği iddia edilmez.

Rust kurulu ortamda README'deki `cargo check` ve `cargo test` komutlarını çalıştırın. Ardından normal/giriş/uyarı durumlarını ve her iki pencereyi varsayılan ve minimum boyutlarında kontrol edin.
