# Markdown & Quarto Translator (마크다운 및 쿼토 번역기)

[🌐 English Version (README.en.md)](README.en.md)

Google Gemini AI를 사용하여 Markdown, MDX 및 Quarto Document (`.qmd`) 파일의 서식과 구조를 완벽하게 보존하면서 다양한 언어로 번역하는 **고성능 Rust 기반 CLI 도구**입니다.

## 주요 기능

- 🌍 **40개 이상의 다국어 지원** - 한국어, 영어, 일본어, 중국어, 스페인어, 프랑스어, 독일어 등 다양한 언어 지원
- 📝 **Markdown, MDX, Quarto (.qmd) 지원** - 헤더, 링크, 코드 블록, 표, 인용구뿐만 아니라 Quarto 전용 구문(YAML Frontmatter, 실행 옵션 `#|`, Callout 블록 `:::`, Shortcode)의 서식을 손상 없이 유지
- 🔄 **스마트 청킹(Smart Chunking)** - 대용량 파일도 컨텍스트 길이에 맞춰 지능적으로 분할 번역
- 🎯 **선택적 번역** - 코드 블록 내 주석 및 캡션은 번역하되, 실행 코드, URL, 파일 경로는 그대로 보존
- 📂 **배치(Batch) 일괄 처리** - Glob 패턴(`docs/**/*.qmd`, `docs/**/*.md`)을 통한 다중 파일 동시 번역
- 🏗️ **디렉토리 구조 유지/평탄화** - 원본 하위 폴더 구조를 그대로 유지하거나 단일 폴더로 평탄화(`--flat`) 가능
- 📊 **실시간 진행률 표시** - `indicatif` 기반의 직관적인 스피너 및 진행률 표시 UI
- ⚡ **빠르고 안정적인 Rust 비동기 엔진** - `tokio` 및 `reqwest` 기반의 빠른 속도와 안정적인 자원 관리

## 설치 및 빌드

### 요구 사항

- Rust 1.70 이상 (`cargo` 포함)
- Google Gemini API 키 ([Google AI Studio에서 무료 발급](https://aistudio.google.com/app/apikey))

### 빌드 및 설치

```bash
# 릴리즈 바이너리 빌드
cargo build --release

# 시스템 전역 PATH에 설치 (어느 폴더에서든 사용 가능하도록 설치)
cargo install --path .
```

컴파일된 바이너리는 `./target/release/md-translate`에 생성됩니다.

> [!TIP]
> `cargo install --path .` 명령어를 실행하면 `~/.cargo/bin/md-translate`에 바이너리가 설치되어 **어느 폴더/경로에서나** `md-translate` 명령어로 바로 실행할 수 있습니다.
> 
> *참고: `~/.cargo/bin` 디렉토리가 시스템의 `PATH` 환경 변수에 등록되어 있어야 합니다 (보통 Rust 설치 시 자동 등록).*
> 
> ```bash
> # 설치 후 다른 폴더에서 바로 사용:
> export GEMINI_API_KEY="your-api-key"  # 또는 전역 환경 변수 설정
> md-translate translate -i file.md -l Korean
> ```

## 환경 설정

### 1. `.env` 파일 설정 (권장)

예제 환경 설정 파일을 복사하여 `.env` 파일을 생성합니다:

```bash
cp env.example .env
```

`.env` 파일을 열어 API 키와 사용할 모델을 설정합니다:

```env
# Google Gemini API 설정
GEMINI_API_KEY=your-google-gemini-api-key-here

# 사용할 Gemini 모델명 (기본값: gemini-2.5-flash)
GEMINI_MODEL=gemini-3.5-flash-lite
```

### 2. 환경 변수 또는 명령줄 옵션 사용

**환경 변수로 설정:**
```bash
export GEMINI_API_KEY="your-api-key-here"
export GEMINI_MODEL="gemini-3.5-flash-lite"
```

**명령줄 옵션으로 직접 전달:**
```bash
./target/release/md-translate translate -i file.md -l Korean --key your-api-key-here --model gemini-3.5-flash-lite
```

## 사용법

### 기본 파일 번역

```bash
# README.md를 한국어로 번역 (README_korean.md 생성)
./target/release/md-translate translate -i README.md -l Korean

# Quarto 문서(.qmd) 번역
./target/release/md-translate translate -i analysis.qmd -l Korean

# 출력 파일 경로를 직접 지정하여 번역
./target/release/md-translate translate -i docs/guide.md -l French -o docs/guide_fr.md

# API 키와 모델을 명령줄 옵션으로 직접 지정
./target/release/md-translate translate -i file.qmd -l German --key your-api-key --model gemini-3.5-flash-lite
```

### 배치(Batch) 다중 파일 번역

Glob 패턴을 사용하여 여러 마크다운/Quarto 파일을 한 번에 번역할 수 있습니다:

```bash
# 현재 디렉토리의 모든 .md 및 .qmd 파일을 한국어로 번역하여 ./korean/ 폴더에 저장
./target/release/md-translate translate -i "*.qmd" -l Korean -d ./korean/

# docs 폴더 및 모든 하위 폴더의 마크다운/Quarto 파일을 디렉토리 구조를 유지하며 번역
./target/release/md-translate translate -i "docs/**/*.qmd" -l Japanese -d ./translations/

# 하위 디렉토리 구조 없이 단일 폴더에 모두 저장 (--flat)
./target/release/md-translate translate -i "content/**/*.{md,qmd}" -l German -d ./output/ --flat

# 접미사(suffix)를 커스텀 지정 (예: sample_ko.qmd)
./target/release/md-translate translate -i "*.qmd" -l Korean -d ./translated/ --suffix "ko"
```

### 지원 명령어

#### `translate` - 마크다운/MDX/Quarto 파일 번역

```bash
./target/release/md-translate translate [옵션]

옵션:
  -i, --input <패턴>       입력 파일 경로 또는 Glob 패턴 (필수)
                          예: "file.md", "document.qmd", "*.qmd", "docs/**/*.md"
  -l, --language <언어>    번역할 대상 언어 (필수, 예: Korean, Spanish, French)
  -o, --output <파일>      출력 파일 경로 (단일 파일 번역 시)
  -d, --output-dir <디렉토리> 출력 디렉토리 (배치 번역 또는 단일 파일)
  -k, --key <API키>        Google Gemini API 키 (.env에 설정 시 생략 가능)
  -m, --model <모델명>     Google Gemini 모델명 (.env에 설정 시 생략 가능, 기본값: gemini-3.5-flash-lite)
      --flat              출력 디렉토리에서 하위 폴더 구조 없이 평탄화
      --suffix <접미사>    출력 파일명 뒤에 붙을 접미사 (기본값: 언어명)
```

#### `languages` - 지원 언어 목록 확인

```bash
./target/release/md-translate languages
```

#### `setup` - 설정 가이드 확인

```bash
./target/release/md-translate setup
```

#### `--help` - 도움말 출력

```bash
./target/release/md-translate --help
```

## 지원 언어

40개 이상의 언어를 지원하며, Google Gemini가 지원하는 모든 언어 이름을 사용할 수 있습니다:

- **아시아**: Korean(한국어), Japanese(일본어), Chinese(중국어), Hindi(힌디어), Thai(태국어), Vietnamese(베트남어), Indonesian(인도네시아어), Malay(말레이어) 등
- **유럽**: English(영어), Spanish(스페인어), French(프랑스어), German(독일어), Italian(이탈리아어), Portuguese(포르투갈어), Dutch(네덜란드어), Russian(러시아어), Polish(폴란드어), Swedish(스웨덴어), Norwegian(노르웨이어), Danish(덴마크어), Finnish(핀란드어), Greek(그리스어), Ukrainian(우크라이나어), Czech(체코어) 등
- **중동**: Arabic(아랍어), Hebrew(히브리어), Turkish(튀르키예어) 등

## 번역 대상 및 보존 규칙

✅ **번역되는 항목**:
- 제목(Heading) 텍스트
- 본문 단락(Paragraph)
- 목록(List) 항목
- 표(Table) 내부 텍스트
- 링크(Link) 텍스트
- 이미지 대체 텍스트(Alt text)
- 인용문(Blockquote)
- 코드 블록 내부의 설명 주석 (`//`, `/* */`, `#` 등)

❌ **원형 그대로 보존되는 항목**:
- 코드 블록 및 인라인 코드의 실행 코드
- URL 링크 및 파일 경로
- 마크다운 문법 기호 (`#`, `*`, `_`, `>` 등)
- HTML 태그
- 수식(LaTeX / Mathjax)
- 고유 명사 및 기술 전문 용어

## 프로젝트 구조

```
markdown-translator/
├── Cargo.toml           # Rust 패키지 설정 및 의존성 관리
├── src/
│   ├── main.rs          # CLI 진입점 및 실행 흐름 제어
│   ├── cli.rs           # Clap 매크로 기반 명령어/옵션 정의
│   └── translator.rs    # Gemini API 연동 및 마크다운 번역 엔진
├── .env                 # 환경 설정 파일 (API Key 및 Model)
├── env.example          # 환경 설정 템플릿 예시
├── examples/            # 번역 테스트용 샘플 마크다운 파일
├── README.md            # 한국어 문서 (현재 파일)
└── README.en.md         # 영문 문서
```

## 라이선스

이 프로젝트는 MIT 라이선스에 따라 라이선스가 부여됩니다. 자세한 내용은 [LICENSE](LICENSE) 파일을 참조하세요.
