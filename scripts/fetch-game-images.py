#!/usr/bin/env python3
"""Sucht freie Screenshots von Open-Source-Spielen auf Wikimedia Commons und lädt Vorschaubilder.

Nur Dateien von Commons (alle frei lizenziert). Schreibt pro Bild Titel, Autor, Lizenz und
Quelle nach credits.json, damit die Webseite die Namensnennung anzeigen kann.
Aufruf: python3 scripts/fetch-game-images.py <ausgabeordner>
"""
import json, os, re, sys, time, urllib.parse, urllib.request

UA = "OpenGameLauncher-site/1.0 (https://github.com/MikeAdamRGB/Open-Game-Launcher)"
API = "https://commons.wikimedia.org/w/api.php"
GAMES = ["0 A.D.", "Veloren", "Xonotic", "SuperTuxKart", "Luanti", "Minetest", "FlightGear",
         "Speed Dreams", "Stunt Rally", "Red Eclipse", "Unvanquished", "Battle for Wesnoth",
         "OpenTTD", "Warzone 2100", "Sauerbraten", "Extreme Tux Racer", "Mindustry", "OpenRA"]

def get(params):
    url = API + "?" + urllib.parse.urlencode({**params, "format": "json"})
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)

def clean(html):
    return re.sub(r"<[^>]+>", "", html or "").strip()[:120]

def main(out):
    os.makedirs(out, exist_ok=True)
    credits = []
    for game in GAMES:
        try:
            data = get({"action": "query", "generator": "search", "gsrnamespace": 6, "gsrlimit": 12,
                        "gsrsearch": f'"{game}" screenshot filetype:bitmap',
                        "prop": "imageinfo", "iiprop": "url|size|extmetadata", "iiurlwidth": 960})
        except Exception as e:  # noqa: BLE001
            print("Fehler", game, e); continue
        pages = sorted((data.get("query") or {}).get("pages", {}).values(), key=lambda p: p.get("index", 99))
        n = 0
        for p in pages:
            title = p.get("title", "")
            if re.search(r"(?i)logo|icon|\.svg|banner|title|menu", title):
                continue
            ii = (p.get("imageinfo") or [{}])[0]
            w, h = ii.get("width", 0), ii.get("height", 0)
            if w < 900 or h < 450 or not (1.2 <= w / max(h, 1) <= 2.4):
                continue
            meta = ii.get("extmetadata") or {}
            slug = re.sub(r"[^a-z0-9]+", "-", game.lower()).strip("-")
            name = f"{slug}-{n + 1}.jpg"
            try:
                req = urllib.request.Request(ii["thumburl"], headers={"User-Agent": UA})
                with urllib.request.urlopen(req, timeout=60) as r, open(os.path.join(out, name), "wb") as f:
                    f.write(r.read())
            except Exception as e:  # noqa: BLE001
                print("Download-Fehler", title, e); continue
            credits.append({"file": name, "game": game, "title": title,
                            "author": clean(meta.get("Artist", {}).get("value")),
                            "license": clean(meta.get("LicenseShortName", {}).get("value")),
                            "source": ii.get("descriptionurl", "")})
            print("ok", name, title)
            n += 1
            time.sleep(0.5)
            if n >= 3:
                break
    with open(os.path.join(out, "credits.json"), "w", encoding="utf-8") as f:
        json.dump(credits, f, ensure_ascii=False, indent=1)

if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "game-images")
