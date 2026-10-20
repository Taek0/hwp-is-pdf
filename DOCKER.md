# Linux Docker CLI 실행

이 이미지는 HWP/HWPX 한 파일을 변환하고 종료하는 Rust CLI다. HTTP 서버, 포트,
큐 또는 워커는 제공하지 않는다. 한컴 뷰어/DLL, Python 변환기, Windows 실행 파일은
필요하지 않다. 변환 PDF와 같은 디렉터리에 `.report.json`, `.qa.json`도 생성한다.

## 빌드와 버전 고정

저장소 루트에서 Linux 컨테이너 모드의 Docker를 사용한다.

```sh
revision=$(git rev-parse HEAD)
image="hwp-engine:0.1.0-container.1-$(git rev-parse --short=7 HEAD)"
docker build --pull --build-arg VCS_REF="$revision" \
  --build-arg IMAGE_VERSION=0.1.0-container.1 -t "$image" .
docker image inspect "$image" --format '{{.Id}}'
```

PowerShell:

```powershell
$revision = git rev-parse HEAD
$image = 'hwp-engine:0.1.0-container.1-' + (git rev-parse --short=7 HEAD)
docker build --pull --build-arg "VCS_REF=$revision" `
  --build-arg IMAGE_VERSION=0.1.0-container.1 -t $image .
docker image inspect $image --format '{{.Id}}'
```

Dockerfile은 Rust **1.99.1 / bookworm**과 **Debian bookworm-slim**의 manifest digest를
고정하고 `cargo build --locked --release --bin hwp-engine`을 실행한다.
`Cargo.lock` 및 `vendor/legacy-chart` 소스/라이선스가 빌드에 포함된다.
`run-cargo.sh`와 호스트 Rust 설치를 사용하지 않는다. 빌드 단계는 Cargo 패키지를
받을 네트워크가 필요하다. 최초 빌드 후 잠긴 버전의 crate와 기본 이미지도 보관해야
오프라인 재빌드가 가능하다. 플랫폼별 이미지/컴파일러 차이까지 같은 바이너리 바이트를
보장한다는 뜻은 아니다. 첫 검증 플랫폼은 `linux/amd64`다.

`.dockerignore`는 Cargo 파일, 엔진 소스, 필요한 vendor 파일만 허용한다.
문서·글꼴·`output/`·`.git`·자격 증명은 빌드 컨텍스트로 보내지 않는다.
런타임에는 Rust 도구 체인과 사용자 글꼴이 없다. 기본 실행 사용자는 `10001:10001`이다.
고정 digest는 자동 보안 업데이트를 받지 않으므로 기본 이미지 교체 시 새 버전으로
빌드하고 문서·글꼴 회귀 검증 후 선택한다.

## 입력, 출력, 글꼴 마운트

Linux/WSL 예시. 경로는 실제 절대 경로로 바꾼다. 출력 디렉터리는 먼저 만들고
호스트 사용자로 실행하면 생성 파일도 같은 UID/GID를 갖는다.

```sh
mkdir -p "$PWD/output/docker"
docker run --rm --network none --read-only \
  --cap-drop ALL --security-opt no-new-privileges \
  --user "$(id -u):$(id -g)" \
  --cpus 2 --memory 2g --memory-swap 2g --pids-limit 64 \
  --mount "type=bind,source=$PWD/samples,target=/input,readonly" \
  --mount "type=bind,source=$PWD/output/docker,target=/output" \
  --mount "type=bind,source=/absolute/path/to/fonts,target=/fonts,readonly" \
  "$image" /input/tables-notes/endnote-01.hwp /output/endnote.pdf /fonts
```

기본 UID/GID `10001:10001`로 실행하려면 `--user`를 생략하고, 출력 디렉터리를
그 사용자에게 쓸 수 있도록 준비한다. Linux에서는 소유권/그룹 ACL로 허용한다.
예: 관리자가 **전용 출력 디렉터리에만** `chown 10001:10001 /srv/hwp-output`을 적용한다.
부모 경로의 탐색 권한도 필요하다. 입력과 글꼴에는 읽기·탐색 권한이 필요하다.
일반 사용자에게 광범위한 `chmod 777`을 적용할 필요는 없다.

Docker Desktop / PowerShell에서 실제 검증한 기본 ENTRYPOINT 명령:

```powershell
$image = 'hwp-engine:0.1.0-container.1-5cf2c00'
New-Item -ItemType Directory -Force output/roadmap/92-linux-container/direct | Out-Null
docker run --rm --network none --read-only `
  --cap-drop ALL --security-opt no-new-privileges `
  --cpus 2 --memory 2g --memory-swap 2g --pids-limit 64 `
  --mount 'type=bind,source=D:\ChatGPT\hwp\hwp-engine\samples,target=/input,readonly' `
  --mount 'type=bind,source=D:\ChatGPT\hwp\hwp-engine\output\roadmap\92-linux-container\direct,target=/output' `
  --mount 'type=bind,source=C:\Windows\Fonts,target=/fonts,readonly' `
  $image /input/tables-notes/endnote-01.hwp /output/endnote.pdf /fonts
```

Windows 바인드 마운트의 권한 동작은 Linux 파일 시스템의 UID/GID 권한과 다르다.
따라서 검증에서는 별도의 임시 Docker 볼륨으로 소유권 허용/거부도 확인했다.
루트로 변환을 실행해서 출력 권한 문제를 우회하지 않는다.

HWPX는 입력 확장자를 그대로 사용한다. 선택 옵션은 마지막에 붙인다.

```sh
# 위와 동일한 마운트/제한 뒤의 CLI 인수 예시
# "$image" /input/example.hwpx /output/example.pdf /fonts --reflow
# "$image" /input/example.hwpx /output/example.pdf /fonts --contour-wrap
```

글꼴 경로 `/fonts`를 항상 명시한다. CLI의 Linux 기본값은 WSL용
`/mnt/c/Windows/Fonts`이므로 컨테이너 기본값으로 사용할 수 없다.
현재 엔진은 지정 디렉터리 바로 아래의 TTF/TTC를 읽는다. 하위 폴더에만 있는 글꼴은
자동 탐색하지 않는다. 같은 파일명·바이트·글꼴 구성이어야 배치 비교가 유효하다.

이번 비교는 `C:\Windows\Fonts`의 TTF/TTC 373개를 그대로 읽기 전용 마운트했다.
Arial, Times New Roman, 맑은 고딕, 바탕, Wingdings, SimSun, MS Gothic,
함초롬바탕/돋움 파일의 존재를 확인했다. `D:\ChatGPT\hwp\Fonts`에도 사용자 글꼴
18개가 있으나, 기존 기준과 같은 구성을 유지하기 위해 이번 비교에는 추가하지 않았다.
글꼴을 교체·추가하면 대체 글꼴 선택과 결과가 달라질 수 있으므로 다시 검증한다.
사용자 글꼴을 저장소나 이미지 레이어에 복사하거나 레지스트리에 게시하지 않는다.

## 자원 및 실행 시간 제한

위 예시의 CPU는 2개 분량, 메모리와 메모리+스왑은 각각 2 GiB, PID는 64개로
제한한다. 변환 중 네트워크를 차단하고 루트 파일 시스템을 읽기 전용으로 실행한다.
입력마다 적절한 제한을 정한다. `--stop-timeout`은 총 실행 시간 제한이 아니다.

이 이미지의 `/usr/bin/timeout`으로 변환 시간을 120초로 제한할 수 있다.
기존 `docker run`의 이미지 앞에 `--entrypoint /usr/bin/timeout`을 넣고 다음처럼 실행한다.

```sh
docker run --rm --network none --read-only \
  --cap-drop ALL --security-opt no-new-privileges \
  --user "$(id -u):$(id -g)" \
  --cpus 2 --memory 2g --memory-swap 2g --pids-limit 64 \
  --mount "type=bind,source=$PWD/samples,target=/input,readonly" \
  --mount "type=bind,source=$PWD/output/docker,target=/output" \
  --mount "type=bind,source=/absolute/path/to/fonts,target=/fonts,readonly" \
  --entrypoint /usr/bin/timeout "$image" --kill-after=5s 120s \
  /usr/local/bin/hwp-engine /input/tables-notes/endnote-01.hwp /output/endnote.pdf /fonts
```

일반적인 시간 초과의 종료 코드는 `124`다. 강제 kill이나 OOM은 `137`이 될 수 있으므로
이때는 컨테이너 상태의 `OOMKilled`도 확인한다. 변환 성공은 `0`, CLI 인수 부족은 `2`다.
시간 초과나 변환 실패 후 PDF/JSON은 불완전할 수 있다. 입력별 임시 출력 디렉터리에
변환하고 종료 코드와 PDF/JSON을 검사한 뒤 결과를 채택한다. `--rm`은 호스트 출력을
삭제하지 않는다. 호스트 측 대기에도 제한을 두면, 이름을 지정한 컨테이너를
`docker rm -f 컨테이너이름`으로 정리해야 한다. 검증 스크립트는 이 정리를 수행한다.

## 비교 재실행

`scripts/validate_container.py`는 검증용 드라이버이며 Python을 변환에 사용하지 않는다.
`pypdf`가 필요하며 이미지 검사에는 Poppler의 `pdftoppm`도 필요하다. 먼저 같은 소스의
Linux CLI를 `cargo build --locked --release --bin hwp-engine`으로 별도 빌드한다.
이번 WSL 빌드에서는 기존 로컬 환경용 `run-cargo.sh`를 이용했다.

```powershell
wsl -d Ubuntu -- /mnt/d/ChatGPT/hwp/hwp-engine/run-cargo.sh build --locked --release --bin hwp-engine
python scripts/validate_container.py `
  --image hwp-engine:0.1.0-container.1-5cf2c00 `
  --manifest output/roadmap/89-native-ole-reference/complete-inputs.json `
  --baseline output/roadmap/91-visual-geometry-audit/after `
  --fonts C:\Windows\Fonts `
  --local-binary /mnt/d/ChatGPT/hwp/hwp-engine/target/release/hwp-engine `
  --wsl Ubuntu --source-root D:\ChatGPT\hwp `
  --output output/roadmap/92-linux-container/new-run --jobs 3 `
  --pdftoppm 'C:\path\to\pdftoppm.exe'
```

`python`은 `pypdf`가 설치된 환경을 지정한다. 이번에 사용한 번들 Python/Poppler의
절대 경로와 실행 명령은 로컬 검증 로그에 기록했다. Linux에서는 `--wsl`을 생략하고
manifest의 입력 경로와 `--local-binary`/`--fonts`/`--source-root`를 Linux 경로로 바꾼다.
`--jobs`는 동시에 비교할 문서 수다(기본 1, 최대 4). 호스트 전체 부하도 고려한다.

원본 및 접근 가능한 복사본의 SHA-256을 대조한 뒤 검증 출력 폴더로 복제해 변환한다.
과거 경로가 사라졌으면 동일 SHA-256의 다른 복사본을 찾는다. 찾지 못한 문서는
`source-unavailable`로 기록하고 성공 건수에 포함하지 않는다. 이전 출력을 덮어쓰지 않는다.
전체를 변환하지 못했거나 PDF/문자/보고서/QA에 차이가 있으면 종료 코드는 `1`이다.
기존의 의도된 QA 소견은 삭제하지 않는다. 처음 3개 문서의 첫 페이지와 PDF가 달라진
문서의 모든 페이지를 이미지로 만든다. 바이트가 같은 PDF는 같은 렌더러에서 같은 이미지가
되므로, 전 페이지 재렌더링을 대신해 엄격한 PDF 바이트 비교를 기록한다.

## 이미지 보관 및 되돌리기

`latest`를 덮어써 운영 버전을 결정하지 않는다. 버전과 Git revision이 들어간 태그,
`docker image inspect`의 완전한 image ID, 사용한 Dockerfile 및 글꼴 구성을 기록한다.
로컬 태그도 이동할 수 있으므로 실제 실행에는 image ID를 지정하면 확실하다.

```sh
# 검증된 image ID를 기록한다. 아래 명령은 로컬 보관용이다.
docker image inspect "$image" --format '{{.Id}}'
docker save -o hwp-engine-verified.tar "$image"
# 복원할 때
docker load -i hwp-engine-verified.tar
# 다음 일회성 변환에서 보관한 이전 태그 또는 완전한 이전 image ID를 선택한다.
```

새 버전을 채택하기 전에 이전 이미지를 보관하고 같은 입력·글꼴·옵션으로 검증한다.
되돌릴 때는 다음 CLI 실행에서 이전 image ID를 지정한다. 원본 문서는 변경하지 않는다.
이전 이미지도 지우는 `docker image prune -a`는 복구용 보관을 확인한 뒤 수행한다.
이번에는 외부 레지스트리 이미지 게시나 운영 환경 배포를 수행하지 않았다.

참고: [Docker 빌드 권장 사항](https://docs.docker.com/build/building/best-practices/),
[Dockerfile 참조](https://docs.docker.com/reference/dockerfile/).

## 추가 엔진 검증 — stage 93

앞 공백의 양쪽 맞춤 오류를 수정한 엔진 `4f497c00273dd66c4174e86f4ac902a76cef0d51`를
`hwp-engine:0.1.0-container.2-4f497c0`로 로컬 빌드했다. 검증용 이미지와 버전 이미지의 엔진
바이너리 SHA-256이 같고, 202종·592쪽의 WSL/Docker PDF·보고서·QA가 모두 일치한다.
버전 이미지에서는 25번 문서도 별도로 재검증했다. 기존 이미지 보관 및 외부 게시 없음.
이미지 ID는 `sha256:dc83aa11009cc84a238f5b724053c61611b9a83311120f1331928d0024da7c6f`이다.
과거 원본 1종은 부재로 제외했다. 상세 범위는
[JUSTIFIED_PADDING.md](reference/JUSTIFIED_PADDING.md)를 참조한다.
