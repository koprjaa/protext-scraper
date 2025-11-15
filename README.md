# protext-scraper

Scraper pro extrakci a analýzu tiskových zpráv z českých PR portálů. Projekt je určen pro akademické účely a umožňuje systematické shromažďování dat z webových stránek pomocí přímého ID skenování.

## Stručný popis

Tento projekt obsahuje nástroj pro scraping tiskových zpráv z portálu Protext.cz. Scraper extrahuje kompletní obsah článků včetně metadat a umožňuje jejich následnou analýzu podle kategorií. Data jsou ukládána ve strukturovaném JSON formátu vhodném pro další zpracování.

Projekt vznikl jako součást akademického výzkumu zaměřeného na analýzu komunikace v českém mediálním prostředí.

## Kontext eseje

Projekt je součástí akademické práce zabývající se analýzou tiskových zpráv v českém prostředí. Práce zkoumá obsahové charakteristiky, kategorizaci a časové rozložení tiskových zpráv publikovaných na veřejných PR portálech. Scraper slouží jako nástroj pro získání reprezentativního datasetu potřebného pro kvantitativní analýzu.

## Jak funguje scraping

Scraper používá kombinovaný přístup:

1. **RSS feed pro zjištění rozsahu**: RSS feed z Protext.cz (`https://www.protext.cz/rss/cz.php`) se používá pouze pro zjištění nejnovějšího ID článku a určení rozsahu pro skenování
2. **Přímé ID skenování**: Hlavní scraping probíhá přímým procházením rozsahu ID článků. Scraper načítá každý článek přímo podle URL `https://www.protext.cz/zprava.php?id={article_id}` a extrahuje jeho obsah

Pro každý článek scraper extrahuje:
- Titulky a hlavní obsah
- Datum publikace
- Kategorii článku
- Metadata (ID, URL)

Scraper respektuje `robots.txt` a používá etické postupy včetně rate limitingu a rotace User-Agentů. Pro anonymní přístup lze použít Tor proxy.

## Co dataset obsahuje

Dataset obsahuje strukturovaná data o tiskových zprávách ve formátu JSON. Každý záznam obsahuje:

- **title**: Název tiskové zprávy
- **content**: Celý textový obsah článku
- **url**: Odkaz na původní článek
- **date**: Datum publikace
- **category**: Kategorie článku (např. "Finance, ekonomika", "IT, telekomunikace")
- **article_id**: Unikátní ID článku z Protext.cz

Dataset není distribuován součástí repozitáře. Uživatel je odpovědný za dodržování právních předpisů při scrapingu.

## Jak probíhá analýza

Scraper obsahuje integrovanou analýzu kategorií (CI - Category Intelligence), která:

1. **Automaticky kategorizuje** načtené články podle jejich zařazení na zdrojovém portálu
2. **Generuje statistiky** o rozložení článků napříč kategoriemi
3. **Umožňuje filtrování** datasetu podle vybraných kategorií
4. **Exportuje analýzu** do samostatného JSON souboru s časovým razítkem

Analýza probíhá automaticky po dokončení scrapingu a výsledky jsou zobrazeny v konzoli i uloženy do souboru.

## Technologie použité v projektu

- **Python 3** - programovací jazyk
- **BeautifulSoup4** - parsování HTML obsahu
- **Requests** - HTTP požadavky
- **lxml** - XML/HTML parser
- **ThreadPoolExecutor** - paralelní zpracování
- **chardet** - detekce kódování
- **JSON** - ukládání strukturovaných dat

## Struktura repozitáře

```
tiskovky scraper/
├── tiskovky_rss_scraper.py    # Hlavní scraper
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
git clone <repository-url>
cd "tiskovky scraper"
```

2. Nainstalujte závislosti:
```bash
pip install -r requirements.txt
```

### Spuštění

Spusťte scraper příkazem:
```bash
python tiskovky_rss_scraper.py
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
- **robots.txt**: Scraper respektuje pravidla `robots.txt` zdrojových webů
- **Rate limiting**: Scraper obsahuje mechanismy pro etické chování (zpoždění mezi požadavky, rotace User-Agentů)
- **Osobní údaje**: Dataset neobsahuje osobní údaje fyzických osob

### Omezení použití

- Nepoužívej scraper pro komerční účely bez souhlasu vlastníků obsahu
- Neobcházej bezpečnostní opatření webových stránek
- Respektuj podmínky použití zdrojových webů
- Dataset není distribuován součástí repozitáře - uživatel je odpovědný za dodržování právních předpisů při scrapingu

### Text a data mining (TDM)

Pro akademické účely lze využít výjimku podle § 39c zákona č. 121/2000 Sb., o právu autorském, která umožňuje reprodukovat díla pro účely textového a datového těžby za podmínek stanovených zákonem.

## Licence

MIT License

Copyright (c) 2024

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Contributing

Příspěvky jsou vítány. Při přispívání prosím:

1. Vytvořte fork repozitáře
2. Vytvořte feature branch (`git checkout -b feature/AmazingFeature`)
3. Commitněte změny (`git commit -m 'Add some AmazingFeature'`)
4. Pushněte do branch (`git push origin feature/AmazingFeature`)
5. Otevřete Pull Request

Ujistěte se, že váš kód dodržuje PEP8 standardy a obsahuje dokumentaci.
