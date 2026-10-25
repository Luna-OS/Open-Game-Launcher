// MAZ Launcher – Webseite: Sprache, Downloads (direkt vom neuesten GitHub-Release), System-Erkennung
(function () {
  "use strict";
  var REPO = "MikeAdamRGB/MAZ-Lauchner";
  var RELEASES = "https://github.com/" + REPO + "/releases/latest";
  var LANGS = ["de", "en", "es", "fr", "it", "pt", "pl", "tr", "ru"];
  var T = window.MAZ_I18N || {};
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
    var saved = store("maz-lang");
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
      "win-x64": function (n) { return n === "MAZ-Launcher.exe"; },
      "win-arm64": function (n) { return n === "MAZ-Launcher-arm64.exe"; },
      "mac-dmg": function (n) { return /-macOS\.dmg$/.test(n); },
      "appimage": function (n) { return n.endsWith("-" + a + ".AppImage"); },
      "deb": function (n) { return n.endsWith("_" + deb + ".deb"); },
      "rpm": function (n) { return n.endsWith("." + a + ".rpm"); },
      "pacman": function (n) { return n.endsWith("-" + a + ".pkg.tar.zst"); },
      "flatpak": function (n) { return n.endsWith("-" + a + ".flatpak"); },
      "snap": function (n) { return n.endsWith("_" + deb + ".snap"); }
    };
    for (var i = 0; i < assets.length; i++) if (tests[kind] && tests[kind](assets[i].name)) return assets[i];
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
    store("maz-lang", e.target.value);
    applyLang(e.target.value);
  });

  applyLang(pickLang());
  fillLinks();

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
