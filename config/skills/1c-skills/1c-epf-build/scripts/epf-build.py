#!/usr/bin/env python3
# epf-build v1.0 — Build external data processor or report (EPF/ERF) from XML sources
# Source: https://github.com/Desko77/claude-code-skills-1c

import argparse
import glob
import os
import random
import shutil
import subprocess
import sys
import tempfile


def resolve_v8path(v8path):
    """Resolve path to 1cv8.exe."""
    if not v8path:
        candidates = glob.glob(r"C:\Program Files\1cv8\*\bin\1cv8.exe")
        if candidates:
            candidates.sort()
            return candidates[-1]
        else:
            print("Error: 1cv8.exe not found. Specify -V8Path", file=sys.stderr)
            sys.exit(1)
    elif os.path.isdir(v8path):
        v8path = os.path.join(v8path, "1cv8.exe")

    if not os.path.isfile(v8path):
        print(f"Error: 1cv8.exe not found at {v8path}", file=sys.stderr)
        sys.exit(1)

    return v8path


# --- Дополнительные аргументы платформы (эталон skills/1c-epf-build) ---
# Список разделяется запятой, а не пробелом: аргумент платформы несет пробел внутри значения
# (/C "имя значение", путь с пробелом), и разбор по пробелу разорвал бы такой аргумент.
def split_platform_arguments(raw):
    if not raw or not raw.strip():
        return []
    return [p.strip() for p in raw.split(",") if p.strip()]


def find_v8_project_file(start_dir):
    """Файл настроек проекта вверх по дереву от каталога исходников."""
    d = os.path.abspath(start_dir)
    for _ in range(20):
        candidate = os.path.join(d, ".v8-project.json")
        if os.path.isfile(candidate):
            return candidate
        parent = os.path.dirname(d)
        if parent == d:
            break
        d = parent
    return None


def project_platform_arguments(start_dir, key):
    import json as _pa_json
    try:
        path = find_v8_project_file(start_dir)
        if not path:
            return []
        with open(path, "r", encoding="utf-8-sig") as f:
            settings = _pa_json.load(f)
        value = settings.get(key)
        if value is None:
            return []
        if isinstance(value, str):
            return split_platform_arguments(value)
        return [str(v) for v in value if str(v)]
    except Exception:
        return []


def resolve_platform_arguments(explicit, start_dir, key):
    """Аргументы вызова заменяют значение из настроек проекта целиком, а не дополняют его.

    None = параметр не задавали (действуют настройки проекта); пустая строка = заданное
    пустое значение (снимает аргументы проекта на этот запуск).
    """
    if explicit is not None:
        return split_platform_arguments(explicit)
    return project_platform_arguments(start_dir, key)


def merge_dash_values(argv, keys):
    """Склеить "-Ключ значение" в "-Ключ=значение" для перечисленных ключей."""
    out = []
    i = 0
    while i < len(argv):
        token = argv[i]
        if token in keys and i + 1 < len(argv):
            out.append(token + "=" + argv[i + 1])
            i += 2
            continue
        out.append(token)
        i += 1
    return out
# --- Конец блока дополнительных аргументов ---


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(
        description="Build external data processor or report (EPF/ERF) from XML sources",
        allow_abbrev=False,
    )
    parser.add_argument("-V8Path", default="", help="Path to 1cv8.exe or its bin directory")
    parser.add_argument("-InfoBasePath", default="", help="Path to file infobase")
    parser.add_argument("-InfoBaseServer", default="", help="1C server (for server infobase)")
    parser.add_argument("-InfoBaseRef", default="", help="Infobase name on server")
    parser.add_argument("-UserName", default="", help="1C user name")
    parser.add_argument("-Password", default="", help="1C user password")
    parser.add_argument("-SourceFile", required=True, help="Path to root XML source file")
    parser.add_argument("-OutputFile", required=True, help="Path to output EPF/ERF file")
    parser.add_argument("-StrictLog", action="store_true",
                        help="Treat rejection patterns in the platform log as errors (elevate exit code to 1)")
    parser.add_argument("-AdditionalV8Arguments", default=None,
                        help="Extra platform arguments, comma-separated (also reach the temp database)")
    args = parser.parse_args(merge_dash_values(sys.argv[1:], ("-AdditionalV8Arguments",)))

    # --- Resolve V8Path ---
    v8path = resolve_v8path(args.V8Path)

    # --- Auto-create stub database if no connection specified ---
    auto_created_base = None
    if not args.InfoBasePath and (not args.InfoBaseServer or not args.InfoBaseRef):
        source_dir = os.path.dirname(os.path.abspath(args.SourceFile))
        auto_base_path = os.path.join(tempfile.gettempdir(), f"epf_stub_db_{random.randint(0, 999999)}")
        stub_script = os.path.join(os.path.dirname(os.path.abspath(__file__)), "stub-db-create.py")
        print("No database specified. Creating temporary stub database...")
        stub_cmd = [sys.executable, stub_script, "-SourceDir", source_dir, "-V8Path", v8path,
                    "-TempBasePath", auto_base_path]
        # Аргументы разрешаются ДО цепочки: заданные только в настройках проекта иначе не
        # дошли бы до создания временной базы.
        stub_extra = resolve_platform_arguments(
            args.AdditionalV8Arguments,
            os.path.dirname(os.path.abspath(args.SourceFile)), "v8args")
        if stub_extra:
            stub_cmd += ["-AdditionalV8Arguments=" + ",".join(stub_extra)]
        result = subprocess.run(stub_cmd, capture_output=False)
        if result.returncode != 0:
            print("Error: failed to create stub database", file=sys.stderr)
            sys.exit(1)
        args.InfoBasePath = auto_base_path
        auto_created_base = auto_base_path

    # --- Validate source file ---
    if not os.path.isfile(args.SourceFile):
        print(f"Error: source file not found: {args.SourceFile}", file=sys.stderr)
        sys.exit(1)

    # --- Ensure output directory exists ---
    out_dir = os.path.dirname(args.OutputFile)
    if out_dir and not os.path.exists(out_dir):
        os.makedirs(out_dir, exist_ok=True)

    # --- Temp dir ---
    temp_dir = os.path.join(tempfile.gettempdir(), f"epf_build_{random.randint(0, 999999)}")
    os.makedirs(temp_dir, exist_ok=True)

    try:
        # --- Build arguments ---
        arguments = ["DESIGNER"]

        if args.InfoBaseServer and args.InfoBaseRef:
            arguments += ["/S", f"{args.InfoBaseServer}/{args.InfoBaseRef}"]
        else:
            arguments += ["/F", args.InfoBasePath]

        if args.UserName:
            arguments.append(f"/N{args.UserName}")
        if args.Password:
            arguments.append(f"/P{args.Password}")

        arguments += ["/LoadExternalDataProcessorOrReportFromFiles", args.SourceFile, args.OutputFile]

        # --- Output ---
        out_file = os.path.join(temp_dir, "build_log.txt")
        arguments += ["/Out", out_file]
        arguments.append("/DisableStartupDialogs")
        settings_dir = os.path.dirname(os.path.abspath(args.SourceFile))
        arguments.extend(resolve_platform_arguments(
            args.AdditionalV8Arguments, settings_dir, "v8args"))

        # --- Execute ---
        print(f"Running: 1cv8.exe {' '.join(arguments)}")
        result = subprocess.run(
            [v8path] + arguments,
            capture_output=True,
            text=True,
        )
        exit_code = result.returncode

        # --- Read log ---
        log_content = ""
        if os.path.isfile(out_file):
            try:
                with open(out_file, "r", encoding="utf-8-sig") as f:
                    log_content = f.read()
            except Exception:
                log_content = ""

        # --- Scan log for silent rejections (эталон: вердикт пакетного запуска, 1.7.0) ---
        # Платформа штатно возвращает 0 при проваленной операции — отказ виден только в журнале.
        fatal_log_patterns = [
            "неверное свойство объекта метаданных",
            "не входит в состав объекта метаданных",
            "неизвестное имя типа",
            "неизвестный объект метаданных",
            "ни один из документов не является регистратором для регистра",
            "неверное значение перечисления",
            "не может быть приведен к типу",
            "необходима версия платформы не меньше",
            "не найден метод",
            "не может быть применен",
        ]
        clean_log_patterns = [
            "ошибок не обнаружено",
            "ошибки не обнаружены",
            "предупреждений не обнаружено",
            "ошибок: 0",
            "предупреждений: 0",
            "errors were not found",
            "0 errors",
        ]
        silent_failures = []
        if log_content:
            for line in log_content.splitlines():
                trimmed = line.strip()
                if not trimmed:
                    continue
                lower = trimmed.lower()
                if any(pat in lower for pat in clean_log_patterns):
                    continue
                for pat in fatal_log_patterns:
                    if pat in lower:
                        silent_failures.append(trimmed)
                        break

        # --- Result ---
        # По умолчанию — вердикт платформы по коду возврата; журнал всегда печатается.
        # С -StrictLog отказ в журнале поднимает код возврата до 1, даже если платформа вернула 0.
        if exit_code == 0:
            print(f"Build completed successfully: {args.OutputFile}")
        else:
            print(f"Error building (code: {exit_code})", file=sys.stderr)

        if log_content:
            print("--- Log ---")
            print(log_content)
            print("--- End ---")

        if silent_failures:
            suffix = "" if args.StrictLog else " (pass -StrictLog to treat as error)"
            print(
                f"[warning] platform reported success, but the log contains "
                f"{len(silent_failures)} problem(s){suffix}",
                file=sys.stderr,
            )
            for f in silent_failures:
                print(f"  {f}", file=sys.stderr)
            if args.StrictLog and exit_code == 0:
                exit_code = 1

        sys.exit(exit_code)

    finally:
        if os.path.exists(temp_dir):
            shutil.rmtree(temp_dir, ignore_errors=True)
        if auto_created_base and os.path.exists(auto_created_base):
            shutil.rmtree(auto_created_base, ignore_errors=True)


if __name__ == "__main__":
    main()
