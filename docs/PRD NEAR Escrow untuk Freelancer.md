# PRD: NEAR Escrow untuk Freelancer

Versi 1.0 · 5 November 2026 · Draft untuk NEAR Hackathon (deadline 14 Nov 2026, 23:59 UTC)

> **Catatan revisi (8 Nov 2026):** dokumen ini adalah PRD awal dan sebagian sudah digantikan dokumen turunan. Perbedaan yang berlaku:
> 1. **Putusan otomatis 50/50 dihapus** (butir F11 dan §5.6). Kalau arbiter tidak memutus dalam 7 hari, dana tetap terkunci dan hanya bisa diselesaikan lewat mutual settlement. Lihat FSD §4.9 dan STS §6.2.
> 2. **Mutual settlement ditambahkan** sebagai fitur P0 baru (F-14) — tidak ada di PRD ini.
> 3. **Rapor** (F12 di sini) dipindah ke P1-02 di FSD, tetapi versi ringkasnya tetap dikerjakan di MVP (SCTS D-04).
> 4. Penomoran fitur berubah: F1–F13 di sini menjadi F-01–F-15 di FSD. Perhatikan dua hal yang berbeda: **F14 di PRD ini = ringkasan sengketa oleh AI**, sedangkan **F-14 di FSD = mutual settlement** — nomornya kebetulan sama, maksudnya tidak.
> 5. Kontrak memakai **pull payment**: dana yang sudah dialokasikan harus ditarik sendiri oleh penerima (STS §9).
>
> Acuan yang berlaku sekarang: FSD v1.4, STS v1.4, SCTS v1.4, SAS v1.4, Test Plan v1.4, dan API & Frontend Integration Specification v1.2.

## 1. Ringkasan

**Escrow** adalah aplikasi "rekber" di blockchain NEAR. Klien menyetor uang ke kontrak pintar, bukan ke rekening orang. Uang baru cair ke freelancer kalau pekerjaan disetujui. Kalau ada perselisihan, uang dibekukan dan seorang penengah (arbiter) memutuskan, dibantu ringkasan dari AI.

Intinya: **tidak ada pihak yang bisa menilep uang, dan tidak perlu saling percaya.** Aturannya ada di kode.

**Track hackathon:** `realworld` (utama), dengan tambahan `agents` bila fitur AI selesai.

## 2. Masalah

| Siapa | Masalah |
| --- | --- |
| Klien | Takut bayar di depan lalu hasil kerja tidak datang atau tidak sesuai. |
| Freelancer | Takut sudah kerja lalu tidak dibayar, atau dibayar lewat platform dengan potongan 3-7% dan kurs yang jelek. |
| Keduanya | Kalau ada sengketa, tidak ada aturan jelas dan tidak ada yang netral. |

Platform freelance besar menyelesaikan ini dengan memotong biaya besar dan menahan kendali atas uang. Kita menawarkan versi yang aturannya transparan dan uangnya tidak dipegang pihak mana pun.

## 3. Target Pengguna

- **Utama:** freelancer (desainer, penulis, developer) yang dibayar klien lintas kota atau lintas negara.
- **Pendamping:** klien yang ingin bayar dengan aman.
- **Arbiter:** pihak netral yang disepakati kedua belah pihak.

## 4. Tujuan dan Bukan Tujuan

**Tujuan (untuk hackathon):**

- Alur escrow dari awal sampai uang cair berjalan penuh di testnet.
- Ada mekanisme sengketa yang jelas dan bisa didemokan.
- Ada "rapor" freelancer yang dihitung dari data on-chain.
- Demo 2 menit yang mudah dipahami juri.

**Bukan tujuan (tidak dikerjakan sekarang):**

- Pembayaran bertahap (milestone).
- Marketplace untuk mencari freelancer atau klien.
- Sistem chat di dalam aplikasi.
- Penarikan ke rekening bank atau e-wallet.
- Biaya platform (0% saat demo).

## 5. Alur Pengguna

1. **Klien membuat proyek:** isi judul, deskripsi, wallet freelancer, wallet arbiter, deadline, masa review, lalu setor uang.
2. **Freelancer menerima atau menolak** lewat link proyek. Kalau menolak, uang langsung kembali ke klien.
3. **Freelancer mengirim hasil** berupa link dan catatan singkat sebelum deadline.
4. **Klien memilih satu dari tiga:**
   - Setujui, uang cair ke freelancer.
   - Minta revisi (maksimal 2 kali), freelancer kerja lagi.
   - Buka sengketa.
5. **Klien diam sampai masa review habis:** freelancer bisa mencairkan sendiri.
6. **Sengketa:** uang dibekukan, kedua pihak kirim bukti, AI merangkum, arbiter memutuskan pembagian. ~~Kalau arbiter tidak memutuskan dalam 7 hari, uang dibagi 50/50.~~ **(digantikan: dana tetap terkunci, hanya mutual settlement — FSD §4.9)**
7. **Selesai:** rapor freelancer dan klien ter-update.

## 6. Status Proyek

`Funded` → `Accepted` → `Delivered` → `Released` (jalur normal)

Jalur lain:

- `Delivered` → `RevisionRequested` → `Delivered` (revisi)
- `Delivered` atau `RevisionRequested` → `Disputed` → `Resolved`
- `Funded` atau `Accepted` → `Refunded` (freelancer menolak, atau deadline lewat)

## 7. Kebutuhan Fungsional

### P0: Wajib ada

| # | Fitur | Aturan penting |
| --- | --- | --- |
| F1 | Buat proyek + setor uang | Nominal lebih dari 0, klien tidak boleh sama dengan freelancer atau arbiter. |
| F2 | Terima atau tolak proyek | Hanya freelancer. Tolak berarti uang kembali ke klien. |
| F3 | Kirim hasil | Hanya freelancer, sebelum deadline. |
| F4 | Setujui hasil | Hanya klien, uang cair ke freelancer. |
| F5 | Cair otomatis saat klien diam | Hanya freelancer, setelah masa review habis. |
| F6 | Refund saat deadline lewat | Hanya klien, hanya jika hasil belum dikirim. |
| F7 | Minta revisi | Maksimal 2 kali, tiap revisi menambah waktu kerja. |
| F8 | Buka sengketa | Klien atau freelancer, hanya setelah hasil dikirim. Uang dibekukan. |
| F9 | Kirim bukti sengketa | Link dan catatan dari kedua pihak. |
| F10 | Putusan arbiter | Arbiter menentukan persentase untuk freelancer (0-100), sisanya ke klien. |
| F11 | ~~Putusan otomatis 50/50~~ **dibatalkan** | Digantikan mutual settlement (FSD F-14). Arbiter yang diam 7 hari tidak memicu pembagian apa pun. |
| F12 | Rapor | Jumlah proyek selesai, total nilai, jumlah sengketa, dihitung dari data kontrak. |
| F13 | Web app | Connect wallet, buat proyek, daftar proyek saya, halaman detail, semua aksi di atas. |

### P1: Kalau sempat

| # | Fitur |
| --- | --- |
| F14 | Ringkasan sengketa oleh AI (rekomendasi saja, keputusan tetap di arbiter). |
| F15 | Pembayaran dengan stablecoin (USDC). |
| F16 | Perpanjang deadline atau batal bersama atas persetujuan kedua pihak. |

### P2: Pengembangan berikutnya

Milestone, biaya platform 1%, notifikasi email atau Telegram, pencarian arbiter profesional.

## 8. Fitur AI (P1)

Saat sengketa dibuka, web app mengirim deskripsi proyek, hasil kerja, dan bukti dari kedua pihak ke model AI. AI mengembalikan:

- ringkasan singkat posisi masing-masing pihak,
- poin yang sesuai dan tidak sesuai dengan deskripsi proyek,
- saran pembagian (misalnya 70/30) beserta alasannya.

**Batasan:** AI hanya memberi saran. Kontrak tidak membaca output AI, jadi tidak ada risiko AI mengendalikan uang. Ini juga disebutkan jelas di antarmuka.

## 9. Kebutuhan Non-Fungsional

- **Keamanan uang:** perubahan status ditulis sebelum uang dikirim, tiap aksi punya pemeriksaan siapa yang boleh, dan tiap status hanya bisa dilewati sekali.
- **Pengujian:** setiap aturan di tabel F1-F12 punya minimal satu tes otomatis, termasuk tes bahwa aksi yang salah ditolak.
- **Kesederhanaan:** pengguna baru bisa membuat proyek dalam kurang dari 2 menit.
- **Bahasa antarmuka:** Inggris untuk juri, dengan teks bantuan singkat yang mudah dipahami.
- **Jaringan:** NEAR testnet untuk pengembangan dan demo.

## 10. Rancangan Teknis Singkat

| Bagian | Pilihan |
| --- | --- |
| Smart contract | Rust dengan near-sdk (sudah ada versi dasar dan 11 tes lulus) |
| Web | React + near-wallet-selector untuk connect wallet |
| AI (P1) | Claude API dipanggil dari web app |
| Penyimpanan bukti | Link dan catatan teks disimpan di kontrak (dibatasi panjangnya) |

**Perubahan yang perlu ditambah pada kontrak dasar:** status `Accepted`, `RevisionRequested`, `Disputed`, `Resolved`, field arbiter, penghitung revisi, fungsi sengketa dan putusan, serta fungsi rapor.

## 11. Jadwal (5-14 November)

| Tanggal | Pekerjaan |
| --- | --- |
| 5-6 Nov | Finalisasi PRD, upgrade kontrak (terima/tolak, revisi, sengketa, arbiter, rapor) |
| 7-8 Nov | Tes kontrak lengkap, deploy ke testnet |
| 9-10 Nov | Web app: connect wallet, buat proyek, daftar proyek, kirim hasil, setujui |
| 11 Nov | Web app: revisi, sengketa, putusan arbiter, rapor |
| 12 Nov | Fitur AI atau stablecoin (pilih satu yang paling siap) |
| 13 Nov | README, rekam video demo, posting di X, repo dibuat public |
| 14 Nov | Submit lebih awal, sisanya cadangan |

## 12. Checklist Submit Hackathon

- Repo GitHub public dengan README yang jelas (cara jalan, arsitektur, apa yang dipakai AI).
- Video demo diposting di X dengan tag @NEARProtocol dan #NEARHackathon.
- Wallet NEAR untuk menerima hadiah.
- Kontrak ter-deploy dan bisa dicoba juri di testnet.
- Cek ulang aturan resmi di situs hackathon sebelum submit.

## 13. Ukuran Keberhasilan

- Alur utama (buat, kirim, setujui) berjalan tanpa error di demo.
- Alur sengketa berjalan sampai uang cair sesuai putusan.
- Semua tes kontrak lulus.
- Juri paham masalah dan solusinya dalam 30 detik pertama demo.

## 14. Risiko dan Cara Mengatasi

| Risiko | Cara mengatasi |
| --- | --- |
| Waktu terlalu mepet | Kerjakan P0 dulu. P1 hanya dikerjakan kalau P0 selesai dan stabil. |
| Bug di logika uang | Banyak tes otomatis dan jangan pakai mainnet untuk demo. |
| Arbiter bisa curang | Disepakati kedua pihak di awal, ada batas waktu, dan di versi berikutnya bisa diganti dewan arbiter. |
| Klien awam tidak punya wallet | Jelaskan alur wallet dengan singkat dan siapkan akun demo. |
| Dianggap "rekber biasa" | Tonjolkan tiga pembeda: tanpa pihak ketiga yang memegang uang, mekanisme sengketa transparan, dan rapor on-chain. |

## 15. Skenario Demo (2 menit)

1. **Masalah (15 detik):** Budi, desainer di Bandung, dikontrak klien di Jerman. Dia takut tidak dibayar, kliennya takut ditipu.
2. **Alur normal (30 detik):** klien setor, Budi kirim hasil, klien setujui, uang cair.
3. **Sengketa (60 detik):** proyek kedua, klien tidak puas. Sengketa dibuka, AI merangkum bukti, arbiter memutuskan 70/30, uang cair sesuai putusan.
4. **Rapor (15 detik):** profil Budi menampilkan riwayat proyek yang tidak bisa dipalsukan.

## 16. Pertanyaan Terbuka

- Apakah arbiter mendapat upah kecil dari nominal proyek?
- Apakah klien dan freelancer boleh mengganti arbiter di tengah proyek?
- Apakah stablecoin atau AI yang lebih diprioritaskan kalau waktu hanya cukup untuk satu?
