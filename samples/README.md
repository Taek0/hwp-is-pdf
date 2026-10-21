# 변환·회귀 검사 샘플

HWP 5.x와 HWPX 파싱, 페이지 배치, PDF 출력을 확인하기 위한 문서 모음입니다.
공개 저장소에서 가져온 문서와 특정 기능을 재현하기 위해 만든 HWPX가 포함되어 있습니다.
전체 검증에는 이 폴더 외의 로컬 사용자 문서와 한컴 대조 PDF도 사용합니다.

## 폴더 구성

| 폴더 | 주요 검사 대상 | 예시 파일 |
|---|---|---|
| [web](web/) | 공개 HWP의 기본 문단·표·글상자와 실제 공문 | `paragraphs.hwp`, `government-notice.hwp` |
| [additional](additional/) | 정렬, 다단, 용지 설정, 표 캡션, 밑줄, 그림 | `multicolumns.hwp`, `table-caption.hwp` |
| [features](features/) | 수식, 차트, OLE, 그룹 도형, 필드, 각주, 배포용 문서 | `equation.hwp`, `group.hwp` |
| [graphics](graphics/) | 곡선·도형, 그림 회전과 자르기 | `shapes.hwpx`, `pic-crop-01.hwp` |
| [typography](typography/) | 글꼴 선택과 글머리표 | `facename2.hwp`, `lists-bullet.hwp` |
| [text-style](text-style/) | 강조점, 장식선, 위·아래첨자, 글자 위치, 빈 문단 | `scripts-offsets.hwpx`, `decorations.hwpx` |
| [tables-notes](tables-notes/) | 중첩·병합 표, 표 분할, 각주·미주 | `inner-table-01.hwp`, `footnote-01.hwp` |
| [overflow](overflow/) | 긴 각주와 페이지보다 큰 표·개체의 다음 페이지 연결 | `long-footnote.hwpx`, `oversized-nested-table.hwpx` |
| [reflow](reflow/) | 줄바꿈·페이지 재배치, 다단 전환, 개체 감싸기, 캡션·탭 | `plain-no-cache.hwpx`, `mixed-two-column.hwpx` |

## 샘플 실행

저장소 루트에서 실행합니다. `/path/to/fonts`는 실제 글꼴 폴더로 바꿉니다.
Windows에서는 `C:/Windows/Fonts`, WSL에서는 `/mnt/c/Windows/Fonts`를 사용할 수 있습니다.

```sh
cargo run --release -- samples/web/paragraphs.hwp output/samples/paragraphs.pdf /path/to/fonts
cargo run --release -- samples/graphics/shapes.hwpx output/samples/shapes.pdf /path/to/fonts
cargo run --release -- samples/reflow/plain-no-cache.hwpx output/samples/plain-reflow.pdf /path/to/fonts --reflow
```

PDF 옆에 `*.report.json`(변환 경고·배치 정보)과 `*.qa.json`(겹침·영역 이탈 등 검사 결과)이 생성됩니다.
기본 모드는 원본의 저장된 줄 배치를 활용하고, `--reflow`는 지원되는 콘텐츠의 줄과 페이지 배치를 다시 계산합니다.
세부 옵션과 컨테이너 실행은 [루트 README](../README.md)와 [Docker 설명](../DOCKER.md)을 참고하세요.

## 테스트와 자료 관리

```sh
cargo test --test reflow
cargo test --test graphics_regression
cargo test --all-targets
```

테스트별로 필요한 글꼴과 로컬 문서 경로가 다를 수 있습니다. 전체 통합 테스트의 일부는
이 저장소에 포함되지 않은 사용자 문서에 의존하므로, 필요한 조건은 [tests](../tests/)의 해당 테스트를 확인하세요.

- 공개 문서의 출처는 각 폴더의 `README.md`와 `sources.json`에 기록되어 있습니다.
  `sha256.json`이 있는 폴더는 원본 해시도 제공합니다. 재사용 시 원 출처의 이용 조건을 확인하세요.
- `*.reference.xml` 등은 구조 비교 자료이며 변환 입력이나 한컴 출력 PDF가 아닙니다.
- 원본 HWP/HWPX는 보존하고, 생성한 PDF·이미지·로그는 Git에서 제외되는 `output/` 아래에 둡니다.
- 하위 README의 페이지 수와 검사 결과는 작성 당시의 기록입니다. 최신 결과는
  [현재 검증 요약](../reference/CURRENT_CORPUS_VALIDATION.md), 지원 범위는
  [FORMAT_COVERAGE.md](../FORMAT_COVERAGE.md)를 참고하세요.

변환 성공과 자동 검사 통과만으로 한컴과 동일한 글꼴·줄 위치·페이지 배치를 보장하지는 않습니다.
