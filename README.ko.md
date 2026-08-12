# murex

[English](README.md) | 한국어

코딩 에이전트를 위한 Boehm 나선형 모델 지휘자입니다. Claude Code 또는 Codex
플러그인으로 설치하면 에이전트가 리스크 주도 루프의 지휘자가 됩니다: 모르는
것을 등록하고, 사이클마다 가장 큰 리스크를
[Ouroboros](https://github.com/Q00/ouroboros)(`ooo auto`)로 스파이크하고,
커밋먼트 리뷰로 게이트를 통과시키고, 노출이 소진될 때까지 반복합니다.
Rust, 단일 바이너리.

이름은 고대에 티리언 퍼플 염료를 뽑아내던 나선 껍데기 고둥에서 왔습니다 —
염료를 얻으려면 껍데기를 깨야 했습니다. 이 도구가 명시하는 거래가 바로
그것입니다: 사이클 하나를 쓰지 않고는 배울 수 없는 것을 배우기 위해
사이클 하나를 씁니다.

## 동작 방식

```
에이전트(당신) ── murex start / risk add     모르는 것을 등록하고 점수화
     │
     ├─ murex cycle ──────────────▶ 최대 노출 리스크의 스파이크 브리프
     ├─ ooo auto "<instruction>" ─▶ Ouroboros가 스파이크를 실행
     ├─ murex commit ─────────────▶ continue | pivot | stop, 비용, 증거
     │
     └─ remaining_exposure가 0에 도달하거나 commit이 stop을 결정할 때까지 반복
```

세 부분이 각자의 일을 맡습니다. `murex`는 결정론적 장부입니다 — 리스크
등록부, 노출 순위, 커밋먼트 게이트 — 그리고 절대 직접 일하지 않습니다.
Ouroboros는 각 스파이크가 통과하는 실행 엔진입니다. 에이전트는
지휘합니다: 사람을 인터뷰해 점수화된 등록부를 만들고, 브리프를 엔진에
넘기고, 증거를 검증하고, 게이트를 닫습니다.

Ouroboros의 `evolve` 루프가 **품질 주도**(평가 게이트를 통과할 때까지
재생성)라면, 이 루프는 **리스크 주도**입니다: 각 사이클은 가장 큰 리스크를
없애기 위해 존재하고, 사이클 사이의 리뷰가 다음 사이클이 비용값을 하는지
결정합니다.

## 설치

```bash
./install.sh
```

Ouroboros가 없으면 설치하고(`uv tool install ouroboros-ai`), `murex`
바이너리를 빌드·설치하고, Ouroboros에 등록하고, Codex가 있으면 스킬을
`~/.codex/skills`에 링크합니다. `cargo`가 필요하고, Ouroboros를 설치해야
할 때만 `uv`가 필요합니다. 둘 중 하나라도 없으면 추측하는 대신 안내와
함께 실패합니다.

**Claude Code** — 플러그인으로 설치하면 스킬이 함께 배포됩니다:

```
/plugin marketplace add janek-moon/murex
/plugin install murex@murex
```

바이너리 자체는 `install.sh`(또는
`cargo install --git https://github.com/janek-moon/murex`)로 설치합니다.

**Codex** — `install.sh`가 `skills/murex`를 `~/.codex/skills/murex`에
링크합니다. Codex는 Claude Code와 같은 SKILL.md 형식을 읽습니다.

같은 과정을 손으로 하면:

```bash
uv tool install ouroboros-ai        # `ooo`가 아직 없을 때만
cargo install --path .              # `murex`를 PATH에 올림
ouroboros plugin discover .         # 매니페스트 검사, 아무것도 쓰지 않음
ouroboros plugin install .          # 선택: `ooo murex` 표면을 추가
```

## 사용

스킬(`skills/murex/SKILL.md`)이 에이전트에게 전체 루프를 가르칩니다.
손으로 하면 이렇습니다:

```bash
murex start "실시간 협업 편집 출시"
murex risk add "CRDT 메모리가 2GB 한도를 넘을 수 있음" --probability 0.6 --impact 0.9
murex cycle                          # -> 최대 노출 리스크의 스파이크 브리프
ooo auto "<브리프의 instruction>"     # Ouroboros가 스파이크를 실행
murex commit --decision continue --cost 1.5 --resolve R1 --evidence "RSS 380MB"
murex status                         # 반경 + 잔여 노출
```

모든 명령은 `--root <repo>`를 받습니다(기본 `.`). Ouroboros에 등록하면
같은 명령을 `ooo murex <cmd>`로도 쓸 수 있습니다.

## 구성

| 경로                      | 역할                                           |
|---------------------------|------------------------------------------------|
| `.claude-plugin/`         | Claude Code 플러그인 + 마켓플레이스 매니페스트 |
| `skills/murex/SKILL.md`   | 에이전트 표면 (Claude Code, Codex)             |
| `src/lib.rs`              | 컨트롤러 로직 — 등록부, 순위, 게이트           |
| `src/main.rs`             | CLI 엔트리포인트; argv 입력, JSON 출력          |
| `ouroboros.plugin.json`   | Ouroboros UserLevel 플러그인 매니페스트        |
| `install.sh`              | 호스트 인지 설치 스크립트                      |
| `tests/spiral.rs`         | 자체 검사: `cargo test`                        |

Ouroboros 매니페스트가 `ouroboros.plugin.json`이라는 파일명을 유지하는
것은 그 플러그인 계약이 강제하기 때문입니다. 상태는 대상 저장소의
`.murex/spiral.json`에 평문 JSON으로 기록되므로, 등록부는 리뷰에서 읽고
diff할 수 있는 상태로 남습니다.

## 범위

리스크 점수는 사람의 판단이며 `risk add`로 입력합니다. 이 플러그인은
확률을 추측하려고 모델을 호출하지 않습니다 — 결정론적 장부와 게이트만
맡습니다. 그래서 에이전트가 리스크에 대해 무엇을 주장하든, 리스크를 닫을
때 기록된 증거에 대조해 감사할 수 있는 상태로 남습니다.
