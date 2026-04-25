#!/bin/bash

# Функция для конвертации изображения в base64 (чистим всё лишнее)
img_to_base64() {
    local file_path="$1"
    if [[ ! -f "$file_path" ]]; then
        echo "Ошибка: Файл '$file_path' не найден." >&2
        return 1
    fi
    local mime_type=$(file -b --mime-type "$file_path")
    # Удаляем любые переносы строк и возвраты каретки из base64
    local b64_data=$(base64 < "$file_path" | tr -d '\n' | tr -d '\r')
    echo "data:$mime_type;base64,$b64_data"
}

# Функция замены из файла (с полной очисткой данных)
replace_from_file() {
    local TARGET_FILE="$1"
    local LABEL="$2"
    local DATA_FILE="$3"

    if [ ! -f "$TARGET_FILE" ] || [ ! -f "$DATA_FILE" ]; then
        echo "Ошибка: Файлы не найдены."
        return 1
    fi

    export TEMP_LABEL="$LABEL"
    # Perl: 1. Читаем файл 2. Удаляем из него переносы строк 3. Делаем замену
    perl -i -pe 'BEGIN{undef $/; open f, "<", "'"$DATA_FILE"'"; $v=<f>; $v =~ s/\R//g; close f} s/\Q$ENV{TEMP_LABEL}\E/$v/g' "$TARGET_FILE"

    echo "${LABEL} заменена"
    unset TEMP_LABEL
}

mkdir -p ./tmp/tempapp
cp ../client/* ./tmp/tempapp/

mkdir -p ./tmp/mod-admin-term
cp ../admin-term/* ./tmp/mod-admin-term

# Обработка файлов для assets.ts
img_to_base64 ../client/logo.png > ./tmp/val.tmp && replace_from_file ./tmp/tempapp/assets.ts "##LOGO##" ./tmp/val.tmp
cat ../client/style.css | tr -d '\r\n' | sed 's/  */ /g' > ./tmp/val.tmp && replace_from_file ./tmp/tempapp/assets.ts "##STYLE##" ./tmp/val.tmp
cat ../client/home-body.html | tr -d '\r\n' | sed 's/  */ /g' > ./tmp/val.tmp && replace_from_file ./tmp/tempapp/assets.ts "##HOME_BODY##" ./tmp/val.tmp
img_to_base64 ../client/like.svg > ./tmp/val.tmp && replace_from_file ./tmp/tempapp/assets.ts "##LIKESVG##" ./tmp/val.tmp
img_to_base64 ../client/comment.svg > ./tmp/val.tmp && replace_from_file ./tmp/tempapp/assets.ts "##COMMENTSVG##" ./tmp/val.tmp
img_to_base64 ../client/login.svg > ./tmp/val.tmp && replace_from_file ./tmp/tempapp/assets.ts "##LOGINSVG##" ./tmp/val.tmp
cat ../admin-term/style.css | tr -d '\r\n' | sed 's/  */ /g' > ./tmp/val.tmp && replace_from_file ./tmp/mod-admin-term/assets.ts "##STYLE##" ./tmp/val.tmp

# Сборка JS
esbuild ./tmp/tempapp/app.ts --bundle --minify --sourcemap --target=es6 --outfile=./app.min.js
# Сборка терминального модуля
esbuild ./tmp/mod-admin-term/mod.ts --bundle --minify --target=es6 --outfile=./terminal.min.js

# Обработка лоадера (GIF)
img_to_base64 ./tmp/tempapp/catgirl.gif > ./tmp/gif_b64.tmp
replace_from_file ./tmp/tempapp/loader.html "##CATGIRLGIF##" ./tmp/gif_b64.tmp

cp ./tmp/tempapp/loader.html ./loader.min.html

# Очистка
rm -f ./tmp/val.tmp ./tmp/gif_b64.tmp
rm -rf ./tmp/tempapp

cargo run
