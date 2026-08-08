#!/bin/bash
# add-prefix-to-folders.sh - добавляет указанный префикс к именам всех папок в текущей директории.
# По умолчанию префикс "1c-".

PREFIX="${1:-1c-}"   # первый аргумент или "1c-" по умолчанию

# Режим "сухой прогон" (dry-run) – только показать, что будет сделано
DRY_RUN=false
if [[ "$2" == "--dry-run" ]]; then
    DRY_RUN=true
fi

# Находим все подпапки (только каталоги) в текущей папке
for dir in */ ; do
    # Убираем завершающий слеш
    oldname="${dir%/}"
    newname="$PREFIX$oldname"
    # Проверяем, не существует ли уже папка с новым именем
    if [[ -e "$newname" ]]; then
        echo "⚠️  Папка '$newname' уже существует, пропускаем '$oldname'"
        continue
    fi
    if [[ "$DRY_RUN" == true ]]; then
        echo "Будет переименовано: '$oldname' -> '$newname'"
    else
        mv "$oldname" "$newname"
        echo "Переименовано: '$oldname' -> '$newname'"
    fi
done

if [[ "$DRY_RUN" == true ]]; then
    echo -e "\nЗапустите без параметра --dry-run для фактического переименования."
fi