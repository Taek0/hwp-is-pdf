# hwp-is-pdf

본 제품은 한컴의 HWP 문서 파일(.hwp) 공개 문서를 참고하여 개발하였습니다.

HWP 5.x와 HWPX를 직접 읽어 PDF를 만드는 Rust 엔진입니다. 한컴 뷰어의 실행 파일이나
DLL을 호출하지 않습니다. 글꼴은 지정한 디렉터리에서 읽으며 저장소에 재배포하지 않습니다.
암호 문서, 구형 HWP 2.x/3.x, 원본 편집은 현재 범위에서 제외합니다.

## 현재 검증 결과 — 2026-10-20

- 기존 203종 중 현재 원본 확보 가능한 202종 변환: 592쪽, 실패 0건. 과거 사본 1종은 원본 부재로 제외.
- 변경 문서 161쪽 이미지 재생성·검사, 검사한 원본 복사본 454개 해시 유지, 미인식 배치 개체 종류·누락 글리프 경고 0건.
- 전체 Rust 테스트 337개와 all-target Clippy 통과.
- 독립 대조 PDF 64개·249쪽의 페이지 수 일치. 불완전한 대조본과 원본 손상은 별도 표시합니다.
- 원본에도 있는 기하 경고 4종은 삭제하지 않고 대조 근거와 함께 보존합니다.
- 차트·OLE 수식/그림, 그룹 좌표, 병합 표·각주/미주, 공백 축소, 문단 테두리와 그림 기준점을 개선했습니다.
- 중간보고서 50쪽의 쪽별 한글 분포는 한컴과 일치하며, 연구 표는 46–47쪽을 유지합니다.

[최신 검증 요약](reference/CURRENT_CORPUS_VALIDATION.md)과
[파일별 검증 스냅샷](reference/verification-stage93.json)에 검사 범위와 제약을 구분했습니다.
전수 검사 통과는 전체 HWP 포맷의 95% 또는 한컴과의 픽셀 단위 일치를 뜻하지 않습니다.
형식 지원 범위는 [FORMAT_COVERAGE.md](FORMAT_COVERAGE.md)를 참고하세요.

## 성능 실험 — 2026-10-20

202개 문서·592쪽의 기존 검증 로그에서 집계한 문서당 실행 시간입니다.
Rust 릴리스 빌드를 사용했으며 프로세스/컨테이너 시작, 글꼴 로딩, PDF·검사 보고서 저장을 포함합니다.

| 환경 | 평균 | 중앙값 | 95백분위 |
|---|---:|---:|---:|
| 로컬 WSL | 3.39초 | 3.56초 | 4.47초 |
| Docker | 3.56초 | 3.76초 | 4.66초 |

별도로 대표 문서 5개를 WSL에서 순차 변환하며 `/usr/bin/time`으로 측정한 최대 RSS는
**70~442MiB**였습니다. 복학원서 2쪽은 154MiB, 언어 기출문서 15쪽은 251MiB,
중간보고서 50쪽은 442MiB를 사용했습니다.

시간은 병렬 검증 중의 경과 시간이고, 메모리는 별도 순차 실행의 엔진 프로세스 최대 RSS입니다.
Docker 전체 메모리나 서버 처리량을 측정한 값은 아니며 CPU·디스크·글꼴·문서 구성에 따라 달라집니다.
시간 로그는 `output/roadmap/93-justified-padding/container/logs/`, 메모리 기록은
`output/memory-benchmark-20261020/results.json`에 있습니다. 이 로컬 산출물은 Git에 포함되지 않습니다.

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
Microsoft 사설 영역의 체크 표시·원형 숫자에는 Wingdings 글꼴이 필요합니다.
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

Linux 컨테이너에서는 다단계 [Dockerfile](Dockerfile)로 CLI를 빌드합니다.
입력·출력 볼륨, 읽기 전용 글꼴 마운트, 비루트 권한, 자원·시간 제한과 이미지 복구는
[Docker 실행 설명](DOCKER.md)을 참고하세요. 글꼴을 이미지에 포함하지 않으며,
컨테이너 실행 시 글꼴 디렉터리 `/fonts`를 명시합니다.

## 구성과 검증 자료

- [src/README.md](src/README.md): 파서, 배치, 글꼴, PDF 출력의 모듈별 책임.
- `tests/`: 기능·회귀 검사. 일부 통합 검사는 로컬 사용자 문서와 Windows 글꼴이 필요합니다.
- [samples/README.md](samples/README.md): 샘플 폴더별 용도, 출처와 변환·회귀 검사 방법.
- `examples/`: 재현용 HWPX 생성기 및 진단 명령.
- `output/roadmap/15-tables/` ~ `57-synthetic-italic/`: 단계별 PDF·이미지·검증 로그.
- `output/roadmap/57-synthetic-italic/regression-final/`: 최신 53개 문서의 PDF·배치·검사 보고서.
- 같은 단계의 `all-inputs.json`, `all-verification.json`, `page-images/`: 원본 목록·검증 결과·전체 페이지 이미지.
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
