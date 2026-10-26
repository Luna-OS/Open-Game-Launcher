// Open Game Launcher – Webseite: Sprache, Downloads (direkt vom neuesten GitHub-Release), System-Erkennung, Animationen
(function () {
  "use strict";
  var REPO = "MikeAdamRGB/MAZ-Lauchner";
  var RELEASES = "https://github.com/" + REPO + "/releases/latest";
  var LANGS = ["de", "en", "es", "fr", "it", "pt", "pl", "tr", "ru"];
  var T = window.OG_I18N || {};
  var lang = "en";
  var assets = [];
  var version = "";

  function store(k, v) {
    try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch (e) { return null; }
  }
  function tr(key) {
    return (T[lang] && T[lang][key]) || (T.en && T.en[key]) || null;
  }

  // ---------- Sprache ----------
  function pickLang() {
    var q = new URLSearchParams(location.search).get("lang");
    if (q && LANGS.indexOf(q) >= 0) return q;
    var saved = store("og-lang");
    if (saved && LANGS.indexOf(saved) >= 0) return saved;
    var list = navigator.languages || [navigator.language || "en"];
    for (var i = 0; i < list.length; i++) {
      var c = String(list[i]).slice(0, 2).toLowerCase();
      if (LANGS.indexOf(c) >= 0) return c;
    }
    return "en";
  }
  function applyLang(l) {
    lang = l;
    document.documentElement.lang = l;
    document.getElementById("lang").value = l;
    document.querySelectorAll("[data-i18n]").forEach(function (el) {
      var v = tr(el.getAttribute("data-i18n"));
      if (v) el.textContent = v;
    });
    var d = tr("meta_desc");
    if (d) document.querySelector('meta[name="description"]').setAttribute("content", d);
    updateMain();
  }

  // ---------- System ----------
  function detectOS() {
    var ua = navigator.userAgent || "";
    var p = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";
    if (/Win/i.test(p) || /Windows/i.test(ua)) return "windows";
    if (/Mac/i.test(p) || /Mac OS X/i.test(ua)) return /iPhone|iPad/.test(ua) ? "other" : "macos";
    if (/Linux|X11/i.test(p + ua) && !/Android/i.test(ua)) return "linux";
    return "other";
  }
  function detectArm() {
    var ua = navigator.userAgent || "";
    return /aarch64|arm64|armv8/i.test(ua + (navigator.platform || ""));
  }
  var os = detectOS();

  // ---------- Dateien ----------
  function arch() {
    var r = document.querySelector('input[name="arch"]:checked');
    return r ? r.value : "x86_64";
  }
  function find(kind) {
    var a = arch(), deb = a === "aarch64" ? "arm64" : "amd64";
    var tests = {
      // Neuer Name zuerst, alter Name (bis 1.7) als Ausweich
      "win-x64": function (n) { return n === "Open-Game-Launcher.exe"; },
      "win-x64-old": function (n) { return n === "MAZ-Launcher.exe"; },
      "win-arm64": function (n) { return n === "Open-Game-Launcher-arm64.exe"; },
      "win-arm64-old": function (n) { return n === "MAZ-Launcher-arm64.exe"; },
      "mac-dmg": function (n) { return /-macOS\.dmg$/.test(n); },
      "appimage": function (n) { return n.endsWith("-" + a + ".AppImage"); },
      "deb": function (n) { return n.endsWith("_" + deb + ".deb"); },
      "rpm": function (n) { return n.endsWith("." + a + ".rpm"); },
      "pacman": function (n) { return n.endsWith("-" + a + ".pkg.tar.zst"); },
      "flatpak": function (n) { return n.endsWith("-" + a + ".flatpak"); },
      "snap": function (n) { return n.endsWith("_" + deb + ".snap"); }
    };
    for (var i = 0; i < assets.length; i++) if (tests[kind] && tests[kind](assets[i].name)) return assets[i];
    if (tests[kind + "-old"]) return find(kind + "-old");
    return null;
  }
  function fillLinks() {
    document.querySelectorAll("[data-asset]").forEach(function (el) {
      var a = find(el.getAttribute("data-asset"));
      el.href = a ? a.browser_download_url : RELEASES;
      var li = el.closest("li");
      if (li) li.style.opacity = assets.length && !a ? ".55" : "";
    });
    updateMain();
  }
  function updateMain() {
    var btn = document.getElementById("main-dl"), txt = document.getElementById("main-dl-text");
    var kind = { windows: detectArm() ? "win-arm64" : "win-x64", macos: "mac-dmg", linux: "appimage" }[os];
    var label = { windows: "dl_windows", macos: "dl_macos", linux: "dl_linux" }[os];
    if (kind && label) {
      var a = find(kind);
      btn.href = a ? a.browser_download_url : "#download";
      txt.textContent = tr(label) || txt.textContent;
    } else {
      btn.href = "#download";
      txt.textContent = tr("dl_generic") || "Download";
    }
    var v = document.getElementById("version-line");
    if (version) v.textContent = (tr("version") || "Version {v}").replace("{v}", version) + " · " + (tr("free_os") || "");
  }

  // ---------- Reiter ----------
  function selectTab(name) {
    ["windows", "macos", "linux"].forEach(function (n) {
      var on = n === name;
      document.getElementById("tab-" + n).setAttribute("aria-selected", on ? "true" : "false");
      document.getElementById("p-" + n).hidden = !on;
    });
  }
  document.querySelectorAll(".os-tabs button").forEach(function (b) {
    b.addEventListener("click", function () { selectTab(b.id.replace("tab-", "")); });
  });
  document.querySelectorAll('input[name="arch"]').forEach(function (r) { r.addEventListener("change", fillLinks); });
  if (os === "macos" || os === "linux") selectTab(os);
  if (os === "linux" && detectArm()) {
    var arm = document.querySelector('input[name="arch"][value="aarch64"]');
    if (arm) arm.checked = true;
  }

  document.getElementById("lang").addEventListener("change", function (e) {
    store("og-lang", e.target.value);
    applyLang(e.target.value);
  });

  applyLang(pickLang());
  fillLinks();

  // Screenshots groß ansehen (nur Bilder aus dem eigenen img/-Ordner)
  var box = document.getElementById("lightbox");
  var boxImg = document.getElementById("lightbox-img");
  if (box && boxImg && typeof box.showModal === "function") {
    Array.prototype.forEach.call(document.querySelectorAll("[data-zoom]"), function (b) {
      b.addEventListener("click", function () {
        var src = b.getAttribute("data-zoom") || "";
        if (!/^img\/[a-z0-9-]+\.webp$/.test(src)) return;
        boxImg.src = src;
        boxImg.alt = b.getAttribute("aria-label") || "";
        box.showModal();
      });
    });
    box.addEventListener("click", function () { box.close(); });
  }


  // ---------- Animationen ----------
  var reduce = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // Name in Buchstaben teilen: jeder Buchstabe startet in der Mitte und geht nach außen
  function splitWord(el) {
    var text = el.textContent;
    el.textContent = "";
    var spans = [];
    for (var i = 0; i < text.length; i++) {
      var s = document.createElement("span");
      s.className = "ch";
      s.setAttribute("aria-hidden", "true");
      s.textContent = text[i] === " " ? " " : text[i];
      el.appendChild(s);
      spans.push(s);
    }
    var box = el.getBoundingClientRect();
    var mid = box.left + box.width / 2;
    spans.forEach(function (s, i) {
      var r = s.getBoundingClientRect();
      var dx = mid - (r.left + r.width / 2);
      s.style.setProperty("--dx", dx.toFixed(1) + "px");
      s.style.setProperty("--d", String(80 + Math.abs(dx) * 0.35 + i * 12));
    });
  }
  var wm = document.querySelector(".wordmark");
  var started = false;
  function startWordmark() {
    if (started) return;
    started = true;
    if (!reduce) document.querySelectorAll("[data-split]").forEach(splitWord);
    if (wm) wm.classList.add("ready");
  }
  // Erst wenn die Schrift geladen ist (sonst stimmen die Abstände nicht) – höchstens 900 ms warten
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(startWordmark);
  setTimeout(startWordmark, 900);

  // Bilder folgen leicht der Maus (Tiefe je Bild)
  var shots = Array.prototype.slice.call(document.querySelectorAll(".shot"));
  if (!reduce && window.matchMedia("(pointer: fine)").matches) {
    var raf = 0, mx = 0, my = 0;
    window.addEventListener("pointermove", function (e) {
      mx = e.clientX / window.innerWidth - 0.5;
      my = e.clientY / window.innerHeight - 0.5;
      if (raf) return;
      raf = requestAnimationFrame(function () {
        raf = 0;
        shots.forEach(function (el) {
          var d = parseFloat(el.style.getPropertyValue("--d")) || 1;
          var inner = el.firstElementChild;
          inner.style.setProperty("--px", (-mx * 26 * d).toFixed(1) + "px");
          inner.style.setProperty("--py", (-my * 20 * d).toFixed(1) + "px");
        });
      });
    }, { passive: true });
  }

  // Kopfzeile, Lineal rechts, aktiver Abschnitt
  var top = document.getElementById("top");
  var mark = document.getElementById("ruler-mark");
  var links = Array.prototype.slice.call(document.querySelectorAll("#secnav a"));
  var secs = links.map(function (a) { return document.querySelector(a.getAttribute("href")); });
  function onScroll() {
    var y = window.scrollY || 0;
    var max = Math.max(1, document.documentElement.scrollHeight - window.innerHeight);
    if (top) top.classList.toggle("solid", y > 20);
    if (mark) mark.style.top = (2 + (y / max) * 96).toFixed(2) + "%";
    var cur = -1;
    secs.forEach(function (s, i) { if (s && s.getBoundingClientRect().top < window.innerHeight * 0.45) cur = i; });
    links.forEach(function (a, i) { a.classList.toggle("on", i === cur); });
  }
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  fetch("https://api.github.com/repos/" + REPO + "/releases/latest", { headers: { Accept: "application/vnd.github+json" } })
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (j) {
      if (!j) return;
      version = String(j.tag_name || "").replace(/^v/i, "");
      assets = (j.assets || []).filter(function (a) {
        return String(a.browser_download_url).indexOf("https://github.com/" + REPO + "/releases/download/") === 0;
      });
      fillLinks();
    })
    .catch(function () {});
})();
