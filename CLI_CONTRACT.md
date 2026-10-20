# HWP Engine CLI 계약

기록일: 2026-10-20. 근거는 `src/main.rs`, `src/lib.rs`, `src/model.rs`,
`src/hwp/document.rs`, `src/layout.rs`, `src/qa.rs`다. 전달본의 확정 커밋은
전달 폴더 `SOURCE_VERSION.json`을 따른다. 이 문서는 현재 구현을 설명한다.

## 호출

바이너리는 `hwp-engine`, Docker ENTRYPOINT는 `/usr/local/bin/hwp-engine`이다.
한 문서를 동기 변환한 뒤 종료한다. HTTP 서버·상주 워커가 아니다.

```sh
hwp-engine INPUT.hwp OUTPUT.pdf FONT_DIRECTORY
hwp-engine INPUT.hwpx OUTPUT.pdf FONT_DIRECTORY --reflow
cargo run --locked --release -- INPUT.hwpx OUTPUT.pdf FONT_DIRECTORY
```

| 위치 인자 | 현재 구현 |
|---|---|
| INPUT | 존재하는 HWP 5.x/HWPX 파일. 확장자는 대소문자를 구분하지 않음 |
| OUTPUT | PDF 파일 경로. 부모 폴더를 생성하고 기존 파일은 덮어쓸 수 있음 |
| FONT_DIRECTORY | 문법상 선택. 백엔드는 `/fonts` 등 실제 경로를 항상 명시 |

글꼴 기본값은 Windows `C:/Windows/Fonts`, 그 외 `/mnt/c/Windows/Fonts`다.
후자는 WSL용 값이다. 컨테이너에서 생략하지 않는다. TTF/TTC 파일을 지정 폴더 바로
아래에 둔다. 입력과 출력은 파일 경로만 받으며 stdin/stdout 스트림 모드는 없다.
`-`도 스트림 지정자가 아니다. 출력 확장자는 검사하지 않으므로 백엔드는 `.pdf`를 쓴다.

## 옵션과 파서의 한계

| 옵션 | 현재 동작 |
|---|---|
| 없음 | 저장된 배치를 사용하고 지원되는 누락·손상 배치 캐시를 복구 |
| `--reflow` | 지원되는 문단·혼합 콘텐츠 재배치를 강제 |
| `--reflow-text` | `--reflow`의 별칭 |
| `--contour-wrap` | 지원되는 기하 윤곽 감싸기 경로 선택 |
| `--hancom-hide-pcx` | PCX 그림만 생략하고 보고서에 기록. 보안 필터가 아님 |

reflow와 contour를 함께 지정하면 contour 경로가 우선한다. PCX 옵션은 독립적이며
기본값은 원본 그림 보존이다. 파서는 위 옵션을 인자 목록 어디서나 제거한 다음 위치
인자를 사용한다. `--input`, `--output`, `--password`, `--help`, `--version`, 별도의
`--` 옵션 종결자는 없다. 최대 인자 수·알 수 없는 옵션을 엄격히 검사하지 않으며,
네 번째 이후 위치 인자는 무시될 수 있다. 백엔드는 문서화된 옵션과 위치 인자 3개만
허용하고, 쉘 문자열 대신 argv 배열로 실행한다.

## 출력과 완료 시점

`/work/job/result.pdf`를 주면 아래 세 파일을 만든다.

```text
/work/job/result.pdf
/work/job/result.qa.json
/work/job/result.report.json
```

부가 파일명은 OUTPUT의 마지막 확장자를 `qa.json`, `report.json`으로 바꾼 것이다.
순서는 PDF 저장 → QA JSON 저장 → report JSON 저장 → 성공 stdout → 정상 종료다.
완료 여부는 **프로세스 종료 코드 0과 세 파일 검사**로 판단한다. stdout만 먼저 보고
게시하지 않는다. 저장은 원자적 rename이 아니며 fsync 영속화 보장도 없다.
실패·취소 시 일부 파일이나 이전 실행의 파일이 남을 수 있으므로 시작할 때 비어 있는
작업별 출력 폴더를 사용하고 검사 후 결과를 채택한다.

동일 입력·출력 검사는 Path의 단순 비교다. symlink/hardlink·정규화된 경로까지 같은지
보장하지 않으므로 입력을 읽기 전용으로 두고 출력을 별도 폴더로 분리한다.

## stdout, stderr와 종료 코드

성공 stdout은 일반 텍스트 요약 한 줄이다. PDF·JSON·진행률 스트림이 아니다.

```text
2 pages; 3 equations; 0 pictures; 1 table draws; 10 warnings
```

정상 성공의 stderr는 보통 비어 있다. 인수 부족은 사용법과 형식 안내를 stderr에 쓴다.
일반 오류는 `anyhow::Result`의 `Error: ...` 및 원인 설명으로 stderr에 출력한다.
오류 문구·원인 체인을 고정된 기계 판독 스키마로 보장하지 않는다.

| 종료 코드 | 의미 |
|---|---|
| `0` | 세 파일 저장 및 정상 반환. 경고·QA 소견이 있어도 가능 |
| `1` | 일반 변환/I/O 오류 반환. 글꼴 누락·권한 오류에서 실제 확인 |
| `2` | 옵션 제거 후 위치 인자가 2개 미만인 사용법 오류 |
| 그 외 | panic·시그널·실행 환경 등 비정상 종료. 전부 1로 단정하지 않음 |

오류 종류별 고정 코드, 암호 문서 전용 코드, 재시도 가능 여부는 제공하지 않는다.
백엔드가 분류·재시도를 관리하며 stderr 문자열을 유일한 계약으로 삼지 않는다.
경고는 report에 배열로 기록하고 stdout에는 개수만 출력한다.

## JSON 필드와 검사

UTF-8 JSON이며 별도 `schema_version` 필드는 없다. 추가 필드를 허용하면서 사용하는
필드의 형식·범위를 확인한다. 현재 필드는 다음과 같다.

| 파일 | 필드 |
|---|---|
| report | 정수 개수 `pages`, `charts`, `equations`, `pictures`, `tables`; 문자열 배열 `warnings`; 경계 배열 `bounds` |
| bounds 항목 | `page`, `kind`, `id`, `x`, `y`, `w`, `h` |
| QA | `glyphs_checked`, `outside_page`, `text_overlaps`, `border_crossings`, `foreground_image_overlaps`, `background_images_exempted`, `intentional_overlays_exempted` |
| QA finding | `page`, `text`, `other`, `rect`(수치 4개 배열) |

페이지 번호는 1부터 시작하고 배치 단위는 pt다. QA는 내부 배치 좌표를 사용한다.
PDF 파서의 좌표와는 원점·변환을 맞춘 뒤 비교한다. 의도된 겹침 등 기존 소견을 보존한다.
QA가 비었다고 문자 완전성·읽기 순서·한컴 픽셀 일치가 증명되는 것은 아니다.
경고 수가 0이 아니라는 사실만으로 실패 처리하지 않는다.

## 제한·취소·부분 파일

엔진 자체에는 deadline·메모리 한도·취소 API·체크포인트·재개 기능이 없다.
검증에서는 CPU 2 / 메모리 2 GiB / PID 64, 읽기 전용 입력·폰트, 네트워크 차단,
`timeout --kill-after=5s 120s`를 사용했다. 구체적인 마운트 명령은 `DOCKER.md`에 있다.

```sh
# 마운트·사용자·자원 옵션 뒤에 이어지는 실행부
docker run ... --entrypoint /usr/bin/timeout IMAGE --kill-after=5s 120s \
  /usr/local/bin/hwp-engine /input/source.hwp /output/result.pdf /fonts
```

`124`는 timeout 래퍼의 시간 초과 코드다. `137`은 강제 kill/OOM 등 원인을 별도
확인한다. `--stop-timeout`은 총 실행 시간 제한이 아니다. 취소 시 백엔드가 프로세스나
컨테이너를 중단하고 부분 파일을 정리한다. 호스트의 subprocess 대기만 끊으면 컨테이너가
남을 수 있어 컨테이너/task 식별자로 회수해야 한다. Fargate의 종료·메모리 연결은 미검증이다.

## 형식·폰트 지원 한계

- 암호화 HWP는 명시적 오류로 거부하며 비밀번호 입력·해독 옵션이 없다.
  공개 형식의 배포용 HWP 디코딩은 별도 구현이다. HWPX 암호 해독 경로도 없다.
- HWP 2.x/3.x와 다른 확장자는 지원하지 않는다.
- HWP 입력/스트림 64 MiB, HWPX ZIP 항목 10,000개·항목별 해제 64 MiB·전체 해제
  256 MiB 제한이 있다. 이는 서버 전체 메모리 상한을 뜻하지 않는다.
- 손상 문서를 거부하거나 일부 복구·대체 표시와 경고를 남길 수 있다. 모든 미지원 기능이
  fatal 오류인 것은 아니며 성공만으로 원본 유효성·완전성을 보장하지 않는다.
- 41개 폰트 구성으로 접근 가능한 202종·592쪽을 기존 373개 구성과 대조했다.
  새 문서의 모든 글꼴 요구를 보장하지 않는다. 교체 시 페이지·문자·QA·그림을 재검증한다.
- PDF에는 글꼴이 임베딩될 수 있다. 폰트의 사용·제공·임베딩 권리는 엔진 MIT와 별개며
  엔진이 모든 사용권을 자동 판정한다고 간주하지 않는다.

형식 한계는 `FORMAT_COVERAGE.md`, 회귀 범위는 `reference/CURRENT_CORPUS_VALIDATION.md`
및 전달 폴더 검증 요약을 따른다. 과거 원본 1종·2쪽은 미검증이다.

## 백엔드 책임

AWS 권한·S3·보관·DB·작업 상태·동시 실행·취소·재시도·PDF 검증·PDF→이미지 연결은
백엔드가 담당한다. 엔진에는 AWS 자격 증명이 필요 없다. 폰트 버전/해시, 엔진 커밋,
입력 해시, 옵션, 종료 코드, stderr, report/QA를 함께 기록하면 재현할 수 있다.
