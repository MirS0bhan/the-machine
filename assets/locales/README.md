# Locale catalogs

JSON key/value catalogs loaded by `ui-runtime` (`i18n.rs`).

Lookup order:

1. Builtin table for the locale
2. `$THE_MACHINE_LOCALE_DIR/{locale}.json`
3. `/etc/the-machine/locales/{locale}.json`
4. `/workspace/assets/locales/{locale}.json` (dev)

Locale selection: `THE_MACHINE_LOCALE` or `LANG` (language subtag).
