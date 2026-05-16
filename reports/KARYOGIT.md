# 🇮🇩 KARYOGIT

> National Open-Source Aggregation & Incubation Platform for Indonesian Developers

KARYOGIT adalah platform agregator dan inkubator open-source nasional yang membantu developer Indonesia mempublikasikan, menemukan, dan mengembangkan proyek secara terbuka dengan infrastruktur ringan, modern, dan scalable.

KARYOGIT menggunakan:
- GitHub Pages → Frontend statis
- Supabase → Backend utama (database + auth + API)
- Wasabi S3 → Storage file rilis
- GitHub Actions → Otomatisasi sinkronisasi

---

## 🌟 Visi

Membangun ekosistem open-source Indonesia yang inklusif, terstruktur, dan kolaboratif, di mana setiap developer dapat dengan mudah mempublikasikan dan mengembangkan karyanya.

---

## 🏛️ Fungsi Utama

### 📚 1. Etalase Open-Source Nasional
Menjadi katalog pusat untuk proyek open-source Indonesia agar:
- Mudah ditemukan
- Terstruktur berdasarkan kategori dan tag
- Tidak tercecer di berbagai platform

### ⚡ 2. Distribusi Rilis Cepat
File rilis disimpan di Wasabi S3:
- .zip, .tar.gz, .apk, .exe
- Download langsung tanpa bottleneck backend
- Optimized untuk akses cepat di Indonesia

### 🌱 3. Inkubator Developer Pemula
Memberikan ruang bagi:
- Mahasiswa
- Developer pemula
- Side project builder

untuk mempublikasikan karya tanpa kompleksitas deployment.

---

## 🚀 Arsitektur Sistem

GitHub Pages → Frontend katalog & UI  
Supabase → Database + Auth + API utama  
Wasabi S3 → File storage (release assets)  
GitHub Actions → Auto sync & deployment workflow  

---

## 🧠 Supabase sebagai Backend Utama

0

Supabase digunakan untuk:
- Menyimpan metadata proyek
- Authentication developer (opsional)
- API otomatis (REST + realtime)
- Manajemen data proyek open-source

---

## 🧱 Struktur Database Supabase

### Tabel: projects
```sql
id UUID PRIMARY KEY,
name TEXT,
description TEXT,
tags TEXT[],
repo_url TEXT,
download_url TEXT,
author TEXT,
created_at TIMESTAMP DEFAULT NOW(),
status TEXT


---

Tabel: developers

id UUID PRIMARY KEY,
name TEXT,
email TEXT,
github_url TEXT,
bio TEXT,
created_at TIMESTAMP DEFAULT NOW()


---

🚀 Fitur Utama

🧾 1. Upload Proyek Instan

Developer cukup:

Isi form proyek

Upload metadata

Kirim file rilis


Sistem akan:

Simpan ke Supabase

Upload file ke Wasabi S3

Tampil otomatis di katalog



---

🔎 2. Pencarian Berbasis Tag Lokal

Frontend GitHub Pages melakukan fetch dari Supabase dan filter berdasarkan tag seperti:

#UMKM

#SistemDesa

#Edutech

#BahasaDaerah



---

🤝 3. Tombol “Gotong Royong”

Setiap proyek memiliki tombol: Bantu Kembangkan

Fungsi:

Mengarahkan ke repository GitHub

Memudahkan kontribusi (issue, pull request, dokumentasi)



---

🪪 4. Badge “Made in Indonesia”

![KARYOGIT Verified](https://karyogit.id/badge?id=PROJECT_ID)

Fungsi:

Identitas proyek Indonesia

Status terdaftar di ekosistem KARYOGIT



---

🔄 5. Otomatisasi GitHub Actions

Sync data dari Supabase

Update GitHub Pages

Refresh katalog otomatis



---

🧭 Alur Pengguna

👀 Pengunjung

1. Buka KARYOGIT


2. Cari proyek berdasarkan tag


3. Klik repo atau download



🧑‍💻 Developer

1. Login (opsional)


2. Klik “Pamerkan Karya”


3. Isi form proyek


4. Proyek langsung muncul di katalog




---

⚙️ Prinsip Desain

Lightweight frontend (GitHub Pages)

Supabase sebagai backend utama

Cost-efficient architecture

API-first system

Community-driven ecosystem

Indonesia-focused tagging system



---

🔐 Keamanan

Row Level Security (Supabase)

Validasi file upload

Rate limiting API

File type filtering



---

🌍 Dampak

Meningkatkan visibilitas developer Indonesia

Mempercepat kolaborasi open-source lokal

Menjadi katalog nasional proyek teknologi

Mendorong budaya gotong royong digital



---

📌 Status

🚧 Active Development


---

🤝 Kontribusi

Kontribusi terbuka untuk:

UI/UX improvement

Backend optimization

Database schema improvement

Feature development

Documentation



---

📜 Lisensi

MIT License (disarankan)


---

🇮🇩 Penutup

“Dari Indonesia, untuk ekosistem open-source yang lebih terbuka, terstruktur, dan kolaboratif.”
