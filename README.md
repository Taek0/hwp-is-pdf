# hwp-is-pdf

본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

HWP 5.x와 HWPX를 직접 읽어 PDF를 만드는 Rust 엔진입니다. 한컴 뷰어의 실행 파일이나
DLL을 호출하지 않습니다. 글꼴은 지정한 디렉터리에서 읽으며 저장소에 재배포하지 않습니다.
암호 문서, 구형 HWP 2.x/3.x, 원본 편집은 현재 범위에서 제외합니다.

## 현재 검증 결과 — 2026-10-16

- Rust 테스트 183개 및 all-target Clippy 통과.
- 기존 43개 문서·205쪽 재변환: 원본 해시와 페이지·개체 수 유지, 네 종류의 기하 검사 0건.
- 한컴 비교 문서 4개의 페이지 수 일치: 복학원서 2, 법학적성시험 15, 수학 시험지 9, 중간보고서 50.
- 글꼴 부분 포함 전후 76쪽의 1000px 렌더링 픽셀 동일. 비교 PDF 용량 약 96.4~98.1% 감소.
- 최신 수정: 끝 글자 자간, 그림 흑백·밝기·대비, 떠 있는 그림의 바깥 여백. 복학원서 워터마크 위치 오차는 가로 0.08pt·세로 0.001pt 미만입니다. PCX 표시 호환 옵션으로 한컴 출력의 로고 생략도 재현합니다.
- 수학 시험지: `hideFirstEmptyLine`과 줄 간격을 반영해 빈 문단의 단·쪽 연결을 복구했습니다. 26개 문항 중 19개의 시작 위치가 한컴과 0.1pt 이내이며, 14~20번에는 지문 줄바꿈에서 이어지는 약 16pt 차이가 남아 있습니다.

페이지 수나 검사 통과가 한컴과의 완전한 일치를 의미하지는 않습니다.
일부 표의 내부 행 분할·각주 분배와 대체 글꼴·워터마크 표현 차이가 남아 있습니다.
현재 범위는 [FORMAT_COVERAGE.md](FORMAT_COVERAGE.md),
최신 형식 지원은 [reference/EMPHASIS_MARKS.md](reference/EMPHASIS_MARKS.md),
최신 한컴 비교는 [reference/BLANK_COLUMN_FLOW.md](reference/BLANK_COLUMN_FLOW.md),
남은 전체 범위는 [reference/PRIORITY_WORKLIST.md](reference/PRIORITY_WORKLIST.md)를 확인하세요.

## 실행

Rust stable / edition 2024 환경에서:

```sh
cargo run -- input.hwp output.pdf /path/to/fonts
cargo run -- input.hwpx output.pdf /path/to/fonts
cargo run -- input.hwpx output.pdf /path/to/fonts --reflow
cargo run -- input.hwpx output.pdf /path/to/fonts --contour-wrap
cargo run -- input.hwp output.pdf /path/to/fonts --hancom-hide-pcx
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

기본 모드는 저장된 줄 배치를 사용하며, 배치 캐시가 없거나 일부 손상된 구역을 재구성합니다.
`--reflow`는 지원되는 혼합 콘텐츠의 재계산, `--contour-wrap`는 기하 윤곽선 감싸기 옵션입니다.
`--hancom-hide-pcx`는 한컴의 제한된 그림 표시와 비교하기 위해 PCX 그림만 생략하는 옵션입니다.
기본 변환은 원본 그림을 보존하며, 이 옵션은 보안 기능이 아닙니다. 생략은 변환 보고서에 기록됩니다.
지원하지 않는 복합 배치에는 제한 또는 명시적 오류가 있으며 전체 형식 지원을 뜻하지 않습니다.

Windows 기본 글꼴 경로는 `C:/Windows/Fonts`, 현재 WSL 검증 경로는 `/mnt/c/Windows/Fonts`입니다.
실제 사용 글꼴과 함께 Arial, Times New Roman, 맑은 고딕, 바탕 및 기호용 TrueType 글꼴을 준비합니다.
한자·일본어에는 SimSun/MS Gothic, 한컴 전용 기호에는 함초롬바탕/돋움의 설치된 글리프를 사용합니다.
TTC의 개별 면, 실제 굵은 글꼴과 기울임 글꼴을 우선 선택하며 대체 글꼴은 보고서에 기록합니다.

라이브러리 API:

```rust
hwp_engine::convert(input_path, output_path, font_directory)?;
hwp_engine::convert_with_reflow(input_path, output_path, font_directory, true)?;
hwp_engine::convert_with_options(input_path, output_path, font_directory, true, true)?;
hwp_engine::convert_configured(input_path, output_path, font_directory,
    hwp_engine::ConversionOptions { hancom_hide_pcx: true, ..Default::default() })?;
```

`run-cargo.sh`는 이 작업 공간의 WSL 도구 체인용 편의 스크립트입니다.
일반 환경에서는 표준 Cargo를 사용합니다. EXE 배포 패키지는 만들지 않습니다.

## 구성과 검증 자료

- [src/README.md](src/README.md): 파서, 배치, 글꼴, PDF 출력의 모듈별 책임.
- `tests/`: 기능·회귀 검사. 일부 통합 검사는 로컬 사용자 문서와 Windows 글꼴이 필요합니다.
- `samples/`, `examples/`: 공개 예제, 재현용 HWPX 생성기 및 진단 명령.
- `output/roadmap/15-tables/` ~ `34-blank-column-recovery/`: 단계별 PDF·이미지·검증 로그.
- `output/roadmap/34-blank-column-recovery/SUMMARY.html`: 네 한컴 대조본과 수학 시험지의 수정 전·후 비교 및 미해결 범위.
- [reference/IMPLEMENTATION_HISTORY.md](reference/IMPLEMENTATION_HISTORY.md): 이전 단계의 구현·검증 기록.

생성된 PDF/이미지와 빌드 결과는 `output/`, `target/`에 두고 Git에서 제외합니다.
원본 HWP/HWPX는 변환 중 수정하지 않습니다. PDF 옆의 `*.report.json`에는 개체·경고·배치 경계,
`*.qa.json`에는 페이지 밖 출력, 글자 겹침, 테두리 교차, 전경 그림 침범 검사가 기록됩니다.
이 검사는 모든 곡선·읽기 순서·원본 충실도를 증명하지 않습니다.

## 근거와 라이선스

[한컴 공개 형식 자료](https://www.store.hancom.com/support/downloadCenter/hwpOwpml)를 참조합니다.
뷰어에 대한 REA 조사 범위와 한계는 [reference/viewer-analysis/README.md](reference/viewer-analysis/README.md)에 있습니다.
전체 내부 렌더러를 역공학했다는 주장은 하지 않습니다.
차트 파서는 공개 Rust 코드 일부를 사용하며 `vendor/legacy-chart/LICENSE`와 `UPSTREAM.md`에 근거를 보존합니다.
