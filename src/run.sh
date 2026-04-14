#!/bin/bash
# Функция для конвертации изображения в base64
img_to_base64() {
    local file_path="$1"

    # Проверяем, существует ли файл
    if [[ ! -f "$file_path" ]]; then
        echo "Ошибка: Файл '$file_path' не найден." >&2
        return 1
    fi

    # Определяем MIME-тип (нужен для использования в HTML/CSS)
    # В macOS используется 'file -I', в Linux 'file -i'
    local mime_type
    if [[ "$OSTYPE" == "darwin"* ]]; then
        mime_type=$(file -b --mime-type "$file_path")
    else
        mime_type=$(file -b --mime-type "$file_path")
    fi

    # Кодируем файл
    local b64_data
    b64_data=$(base64 < "$file_path" | tr -d '\n')

    # Выводим готовую строку для Data URI
    echo "data:$mime_type;base64,$b64_data"
}

replace_label() {
    LABEL=$2
    FILE=$1
    VALUE=$3

    # Проверка на наличие файла
    if [ ! -f "$FILE" ]; then
        echo "Ошибка: Файл $FILE не найден."
        return 1
    fi

    # Используем sed для замены.
    # В качестве разделителя используем | на случай, если в строке замены есть слеши
    sed "s|$LABEL|$VALUE|g" "$FILE" > "$FILE.tmp" && mv "$FILE.tmp" "$FILE"

    echo "${LABEL} заменена"
}

mkdir -p ./tmp/tempapp
cp ./client/* ./tmp/tempapp/

replace_label ./tmp/tempapp/assets.ts "##LOGO##" $(img_to_base64 ./client/logo.png)
replace_label ./tmp/tempapp/assets.ts "##STYLE##" "$(cat ./client/style.css | tr -d '\n' | sed 's/  */ /g')"
replace_label ./tmp/tempapp/assets.ts "##HOME_BODY##" "$(cat ./client/home-body.html | tr -d '\n' | sed 's/  */ /g')"

esbuild ./tmp/tempapp/app.ts \
  --bundle \
  --minify \
  --sourcemap \
  --target=es6 \
  --outfile=web/app.min.js

rm -rf ./tmp/tempapp

cargo run
