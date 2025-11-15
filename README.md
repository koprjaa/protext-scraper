# protext-scraper

Scraper pro extrakci a analýzu tiskových zpráv z portálu Protext.cz. Projekt je určen pro akademické účely a umožňuje systematické shromažďování dat z webových stránek pomocí přímého ID skenování jako součást automatizovaného vytěžování textu (Text and Data Mining, TDM).

## Stručný popis

Scraper extrahuje obsah článků z portálu Protext.cz včetně metadat a umožňuje analýzu podle kategorií. Data jsou ukládána ve formátu JSON.

## Kontext eseje

Projekt byl vytvořen jako součást eseje v předmětu **4IT550 - Competitive Intelligence** v ZS 2025/2026. Scraper slouží pro získání datové sady tiskových zpráv z portálu Protext.cz potřebného pro kvantitativní obsahovou analýzu (QCA).

## Jak funguje scraping

Scraper používá kombinovaný přístup:

1. **RSS feed pro zjištění rozsahu**: RSS feed z Protext.cz (`https://www.protext.cz/rss/cz.php`) se používá pouze pro zjištění nejnovějšího ID článku a určení rozsahu pro skenování
2. **Přímé ID skenování**: Hlavní scraping probíhá přímým procházením rozsahu ID článků. Scraper načítá každý článek přímo podle URL `https://www.protext.cz/zprava.php?id={article_id}` a extrahuje jeho obsah

Pro každý článek scraper extrahuje:
- Titulky a hlavní obsah
- Datum publikace
- Kategorii článku
- Metadata (ID, URL)

Scraper používá etické postupy včetně rate limitingu, zpoždění mezi požadavky a rotace User-Agentů. Pro anonymní přístup lze použít Tor proxy.

## Co dataset obsahuje

Dataset obsahuje data ve formátu JSON. Tiskové zprávy jsou strukturovány podle obrácené pyramidy (nejdůležitější informace na začátku). Každý záznam obsahuje:

- **title**: Název tiskové zprávy
- **content**: Celý textový obsah článku
- **url**: Odkaz na původní článek
- **date**: Datum publikace
- **category**: Kategorie článku (např. "Finance, ekonomika", "IT, telekomunikace")
- **article_id**: Unikátní ID článku z Protext.cz

## Jak probíhá analýza

Scraper obsahuje analýzu kategorií, která:

1. **Kategorizuje** načtené články podle jejich zařazení na zdrojovém portálu
2. **Generuje statistiky** o rozložení článků napříč kategoriemi
3. **Umožňuje filtrování** datasetu podle vybraných kategorií
4. **Exportuje analýzu** do samostatného JSON souboru

Výsledky jsou zobrazeny v konzoli i uloženy do souboru.

## Technologie použité v projektu

- **Python 3**
- **BeautifulSoup4** - parsování HTML
- **Requests** - HTTP požadavky
- **lxml** - XML/HTML parser

## Struktura repozitáře

```
protext-scraper/
├── main.py                     # Hlavní scraper
├── requirements.txt            # Python závislosti
├── README.md                   # Dokumentace
├── data/
│   └── categories.json         # Seznam kategorií
└── output/                     # Výstupní soubory (generováno při běhu)
    ├── content_YYYYMMDD_HHMMSS.json
    └── categories_YYYYMMDD_HHMMSS.json
```

## Jak spustit scraper

### Požadavky

- Python 3.7 nebo vyšší
- Nainstalované závislosti z `requirements.txt`

### Instalace

1. Naklonujte repozitář:
```bash
git clone https://github.com/koprjaa/protext-scraper.git
cd protext-scraper
```

2. Nainstalujte závislosti:
```bash
pip install -r requirements.txt
```

### Spuštění

Spusťte scraper příkazem:
```bash
python main.py
```

Scraper nabídne interaktivní menu s možnostmi:
- Různé rozsahy ID pro skenování (TEST, SMALL, MEDIUM, LARGE, MASSIVE, MAXIMUM)
- Vlastní rozsah ID
- Analýza kategorií

Výstupy se automaticky ukládají do složky `output/` ve formátu JSON. Při každém novém spuštění se staré reporty automaticky mažou.

### Volitelné: Tor proxy

Pro anonymní přístup můžete použít Tor. Ujistěte se, že máte spuštěný Tor service na `127.0.0.1:9050`. Scraper automaticky detekuje dostupnost Tor připojení.

## Etické a právní upozornění

**Důležité**: Tento scraper je určen výhradně pro akademické a výzkumné účely.

### Právní odpovědnost

- **Autorský zákon**: Respektujte český Autorský zákon (121/2000 Sb.) a výjimky pro text a data mining (TDM)
- **Zákonný přístup**: **Uživatel musí mít k datům zákonný přístup.** Aplikace TDM výjimek je podmíněna tím, že uživatel musí mít k vytěžovaným materiálům zákonný přístup (Lawful Access)
- **Rate limiting**: Scraper obsahuje mechanismy pro etické chování (zpoždění mezi požadavky, rotace User-Agentů, Tor proxy pro anonymní přístup)
- **Upozornění**: Scraper automaticky nekontroluje `robots.txt` - uživatel je odpovědný za dodržování pravidel zdrojového webu
- **Osobní údaje**: Dataset neobsahuje osobní údaje fyzických osob

### Omezení použití

- Nepoužívej scraper pro komerční účely bez souhlasu vlastníků obsahu
- Neobcházej bezpečnostní opatření webových stránek
- Respektuj podmínky použití zdrojového webu

### Text a data mining (TDM)

Pro akademické a výzkumné účely lze využít výjimku podle § 39c (obecné TDM) a § 31 odst. 1 písm. c) nebo § 39d zákona č. 121/2000 Sb., o právu autorském. § 39d se vztahuje na vědecký výzkum a umožňuje uchovávat rozmnoženiny po dobu nezbytnou pro ověření výsledků výzkumu. Pro účely vědeckého výzkumu platí výjimky podle principů GDPR a Autorského zákona ve znění harmonizujícím s EU směrnicemi pro vědecký výzkum (§ 89 GDPR, § 39d AZ).

## Licence

Projekt je licencován pod MIT licencí. Viz soubor [LICENSE](LICENSE) pro detaily.
