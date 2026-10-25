# Server für Konto, Freunde & Chat einrichten

Der MAZ Launcher speichert Konten, Freundeslisten und Chat-Nachrichten auf einem Server.
Dafür nutzen wir **Supabase** – kostenlos, ohne Kreditkarte. Das musst du **nur einmal** machen
(als Entwickler des Launchers). Danach funktioniert alles automatisch für alle, die den Launcher herunterladen.

Dauer: ca. 15 Minuten.

---

## 1. Supabase-Projekt erstellen

1. Geh auf <https://supabase.com> und klicke **Start your project**. Melde dich mit GitHub an.
2. Klicke **New project**.
3. Name: `maz-launcher`. Datenbank-Passwort: irgendein sicheres (brauchst du später nicht). Region: **Central EU (Frankfurt)**.
4. **Create new project** – kurz warten, bis das Projekt fertig ist.

## 2. Datenbank einrichten

1. Links im Menü **SQL Editor** öffnen → **New query**.
2. Den kompletten Inhalt der Datei [`supabase/schema.sql`](../supabase/schema.sql) hineinkopieren.
3. Unten rechts **Run** klicken. Es muss „Success. No rows returned“ erscheinen.

(Die Datei darf man auch später nochmal ausführen, z. B. nach einem Update – es geht nichts verloren.)

## 3. Login-Weiterleitung erlauben

1. Links **Authentication** → **URL Configuration**.
2. Bei **Redirect URLs** auf **Add URL** klicken und eintragen:
   ```
   http://127.0.0.1:47821/callback
   ```
3. Speichern.

## 4. Anmeldung per E-Mail

1. **Authentication** → **Sign In / Providers** → **Email** ist standardmäßig an.
2. Optional: **Confirm email** ausschalten. Dann können sich Leute sofort anmelden, ohne erst den Link in der E-Mail zu klicken.
   (Achtung: Supabase verschickt im Gratis-Tarif nur wenige E-Mails pro Stunde.)

## 5. Anmeldung mit Google (optional)

1. Öffne <https://console.cloud.google.com> → oben ein neues Projekt anlegen (z. B. „MAZ Launcher“).
2. **APIs & Dienste** → **OAuth-Zustimmungsbildschirm**: Typ **Extern**, App-Name „MAZ Launcher“, deine E-Mail eintragen, speichern. Danach **App veröffentlichen**.
3. **APIs & Dienste** → **Anmeldedaten** → **Anmeldedaten erstellen** → **OAuth-Client-ID** → Typ **Webanwendung**.
4. Bei **Autorisierte Weiterleitungs-URIs** eintragen (deine Projekt-ID findest du in der Supabase-Adresse):
   ```
   https://DEINE-PROJEKT-ID.supabase.co/auth/v1/callback
   ```
5. **Erstellen** → Client-ID und Clientschlüssel kopieren.
6. In Supabase: **Authentication** → **Sign In / Providers** → **Google** → aktivieren, Client-ID und Secret einfügen → **Save**.

## 6. Anmeldung mit GitHub (optional)

1. Öffne <https://github.com/settings/developers> → **OAuth Apps** → **New OAuth App**.
2. Application name: `MAZ Launcher`, Homepage URL: `https://github.com/MikeAdamRGB/MAZ-Lauchner`
3. Authorization callback URL:
   ```
   https://DEINE-PROJEKT-ID.supabase.co/auth/v1/callback
   ```
4. **Register application** → **Generate a new client secret**. Client ID und Secret kopieren.
5. In Supabase: **Authentication** → **Sign In / Providers** → **GitHub** → aktivieren, beides einfügen → **Save**.

## 7. Server in den Launcher einbauen

1. In Supabase: **Project Settings** → **API** (bzw. **Data API** und **API Keys**).
2. Kopiere die **Project URL** (`https://….supabase.co`) und den **anon public** Key (lange Zeichenkette, beginnt mit `eyJ…`).
   ⚠️ Niemals den `service_role`-Key verwenden!
3. Auf GitHub im Repository: **Settings** → **Secrets and variables** → **Actions** → **New repository secret**:
   - Name `MAZ_SUPABASE_URL` → Wert: die Project URL
   - Name `MAZ_SUPABASE_KEY` → Wert: der anon Key
4. Unter **Actions** den letzten Build mit **Re-run all jobs** neu starten (oder einfach den nächsten Commit pushen).
   Die neue EXE hat den Server dann fest eingebaut – alle Nutzer können sich sofort anmelden.

**Ohne neuen Build testen:** Im Launcher unter **Einstellungen → Konto → Server** die Project URL und den anon Key eintragen.

---

## Ist das sicher?

- Der anon Key ist öffentlich gedacht – er darf in der EXE stehen.
- Die Datenbank ist so eingerichtet, dass niemand direkt auf die Tabellen zugreifen kann. Alles läuft über geprüfte Funktionen:
  Nachrichten kann nur lesen, wer im Chat ist; schreiben kann man nur Freunden; Online-Status und Spiel sehen nur Freunde.
- Passwörter speichert Supabase verschlüsselt – der Launcher sieht sie nie.

## Probleme?

| Fehlermeldung im Launcher | Lösung |
| --- | --- |
| „Diese Anmeldeart ist auf dem Server noch nicht eingeschaltet“ | Schritt 5 bzw. 6 prüfen – Provider in Supabase aktiviert? |
| Browser zeigt nach Google/GitHub-Login „redirect_to not allowed“ o. Ä. | Schritt 3: die Redirect-URL exakt so eintragen |
| „Bitte bestätige erst deine E-Mail-Adresse“ | Link in der E-Mail klicken oder in Schritt 4 „Confirm email“ ausschalten |
| „Keine Verbindung zum Server“ | Project URL und Key nochmal kopieren; Supabase pausiert Gratis-Projekte nach 7 Tagen ohne Nutzung – im Dashboard auf **Restore** klicken |
