#!/usr/bin/env bash
# Bake self-contained design-system preview cards: marker + inlined styles.css + body.
# Re-run after editing styles.css or any *.body.html fragment.
set -euo pipefail
DS="$(cd "$(dirname "$0")/.." && pwd)"
TOKENS="$DS/../assets/tokens.css"
STYLES="$DS/styles.css"

# Publish a copy of the shared tokens into the gallery dir so the synced
# design-system project carries a real, bindable token file (the project's
# styles.css @imports it). The baked cards still inline the tokens directly;
# this copy is only for the project's token layer. Gitignored — regenerated
# here, never committed.
cp "$TOKENS" "$DS/tokens.css"
echo "copied tokens.css"

bake() {
  local body="$1" out="$2" group="$3" title="$4"
  mkdir -p "$(dirname "$DS/$out")"
  {
    printf '<!-- @dsCard group="%s" -->\n' "$group"
    printf '<!doctype html>\n<html lang="en">\n  <head>\n    <meta charset="utf-8" />\n'
    printf '    <title>%s</title>\n    <style>\n' "$title"
    cat "$TOKENS"
    cat "$STYLES"
    printf '    </style>\n  </head>\n  <body>\n'
    cat "$DS/_bodies/$body"
    printf '  </body>\n</html>\n'
  } > "$DS/$out"
  echo "baked $out"
}

bake foundations.body.html         foundations/foundations.html Foundations "Foundations — tokens"
bake navigation-topbar.body.html   navigation/topbar.html      Navigation "Navigation — top bar, tabs, stats"
bake inputs-controls.body.html     inputs/controls.html        Inputs     "Inputs — toolbar, buttons, toggles"
bake data-timeline.body.html       data/timeline.html          Data       "Data — timeline & detail pane"
bake data-cards.body.html          data/cards.html             Data       "Data — unit & revision-thread cards"
bake data-review-facts.body.html   data/review-facts.html      Data       "Data — verdict, facts, endorsements"
bake data-diff.body.html           data/diff.html              Data       "Data — annotated diff"
bake feedback-diagnostics.body.html feedback/diagnostics.html  Feedback   "Feedback — diagnostics & errors"
