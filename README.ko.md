# murex

[English](README.md) | 한국어

![murex — 나선형 모델 지휘자](assets/banner.png)

코딩 에이전트를 위한 Boehm 나선형 모델 지휘자입니다. Claude Code 또는
Codex 플러그인으로 설치하면 리스크 주도 루프를 구동합니다: 모르는 것을
등록하고, 사이클마다 가장 큰 리스크를
[Ouroboros](https://github.com/Q00/ouroboros)(`ooo auto`)로 스파이크하고,
커밋먼트 리뷰로 게이트를 통과시키고, 노출이 소진될 때까지 반복합니다.
Rust, 단일 바이너리.

## 동작 원리

```
에이전트(당신) ── murex start / risk add     모르는 것을 등록하고 점수화
     │
     ├─ murex cycle ──────────────▶ 최대 노출 리스크의 스파이크 브리프
     ├─ ooo auto "<instruction>" ─▶ Ouroboros가 스파이크를 실행
     ├─ murex commit ─────────────▶ continue | pivot | stop, 비용, 증거
     │
     └─ remaining_exposure가 0에 도달하거나 commit이 stop을 결정할 때까지 반복
```

`murex`는 결정론적 장부입니다 — 리스크 등록부, 노출 순위(확률 × 영향),
커밋먼트 게이트 — 그리고 절대 직접 일하지 않습니다. Ouroboros는 각
스파이크가 통과하는 실행 엔진입니다. Ouroboros의 `evolve` 루프가 품질
게이트를 통과할 때까지 돈다면, 이 루프는 리스크가 소진될 때까지 돕니다.
상태는 대상 저장소의 `.murex/spiral.json`에 평문 JSON으로 기록됩니다.

## 설치

```bash
./install.sh   # Ouroboros가 없으면 설치, murex 빌드, 양쪽 호스트에 등록
```

Claude Code:

```
/plugin marketplace add janek-moon/murex
/plugin install murex@murex
```

Codex는 `install.sh`가 링크해 주는 `~/.codex/skills/spiral`에서 같은
스킬을 읽습니다.

## 사용

스킬(`skills/spiral/SKILL.md`, 호출은 `/murex:spiral`)이 에이전트에게
전체 루프를 가르칩니다. 손으로 하면:

```bash
murex start "실시간 협업 편집 출시"
murex risk add "CRDT 메모리가 2GB 한도를 넘을 수 있음" --probability 0.6 --impact 0.9
murex cycle                          # -> 최대 노출 리스크의 스파이크 브리프
ooo auto "<브리프의 instruction>"     # Ouroboros가 스파이크를 실행
murex commit --decision continue --cost 1.5 --resolve R1 --evidence "RSS 380MB"
murex status                         # 반경 + 잔여 노출
```

모든 명령은 `--root <repo>`를 받습니다(기본 `.`). Ouroboros에 등록하면
(`ouroboros plugin install .`) 같은 명령을 `ooo murex <cmd>`로도 쓸 수
있습니다.
