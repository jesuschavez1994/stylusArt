#!/usr/bin/env sh
# Download the typefaces StylusArt bundles in its APK (assets/fonts), from the google/fonts
# repository. Every family here is under the SIL Open Font License (or Apache 2.0); each one's
# license file is saved next to it. The fonts are not committed (see .gitignore): run this once
# before `cargo apk build`.
set -eu
cd "$(dirname "$0")"
out=assets/fonts
mkdir -p "$out/licenses"
base=https://raw.githubusercontent.com/google/fonts/main

# dir|file (inside the family directory)|license file
fonts='
ofl/montserrat|Montserrat[wght].ttf|OFL.txt
ofl/poppins|Poppins-Regular.ttf|OFL.txt
ofl/poppins|Poppins-Bold.ttf|OFL.txt
ofl/lato|Lato-Regular.ttf|OFL.txt
ofl/lato|Lato-Bold.ttf|OFL.txt
ofl/raleway|Raleway[wght].ttf|OFL.txt
ofl/oswald|Oswald[wght].ttf|OFL.txt
ofl/bebasneue|BebasNeue-Regular.ttf|OFL.txt
ofl/anton|Anton-Regular.ttf|OFL.txt
ofl/playfairdisplay|PlayfairDisplay[wght].ttf|OFL.txt
ofl/abrilfatface|AbrilFatface-Regular.ttf|OFL.txt
ofl/lobster|Lobster-Regular.ttf|OFL.txt
ofl/pacifico|Pacifico-Regular.ttf|OFL.txt
ofl/greatvibes|GreatVibes-Regular.ttf|OFL.txt
ofl/caveat|Caveat[wght].ttf|OFL.txt
ofl/indieflower|IndieFlower-Regular.ttf|OFL.txt
ofl/shadowsintolight|ShadowsIntoLight.ttf|OFL.txt
ofl/amaticsc|AmaticSC-Bold.ttf|OFL.txt
ofl/righteous|Righteous-Regular.ttf|OFL.txt
apache/permanentmarker|PermanentMarker-Regular.ttf|LICENSE.txt
'

echo "$fonts" | while IFS='|' read -r dir file lic; do
  [ -z "$dir" ] && continue
  name=$(basename "$dir")
  enc=$(printf '%s' "$file" | sed 's/\[/%5B/g; s/\]/%5D/g')
  if [ ! -s "$out/$file" ]; then
    curl -fsSL -A "Photocraft-dev" "$base/$dir/$enc" -o "$out/$file" && echo "ok   $file" || { echo "FAIL $file"; rm -f "$out/$file"; }
  fi
  if [ ! -s "$out/licenses/$name-$lic" ]; then
    curl -fsSL -A "Photocraft-dev" "$base/$dir/$lic" -o "$out/licenses/$name-$lic" || rm -f "$out/licenses/$name-$lic"
  fi
done
ls "$out" | grep -c '\.ttf$' | xargs echo "fonts:"
