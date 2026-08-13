# murex

[English](README.md) | 한국어

![murex — 나선형 모델 지휘자](assets/banner.png)

코딩 에이전트를 위한 Boehm 나선형 모델 지휘자입니다. Claude Code 또는
Codex 플러그인으로 설치하면 리스크 주도 루프를 구동합니다: 모르는 것을
등록하고, 사이클마다 가장 큰 리스크를 스파이크하고, 커밋먼트 리뷰로
게이트를 통과시키고, 노출이 소진될 때까지 반복합니다. Rust, 단일 바이너리.

## 동작 원리

```
에이전트(당신) ── murex start / risk add     모르는 것을 등록하고 점수화
     │
     ├─ murex cycle ──────────────▶ 최대 노출 리스크의 스파이크 브리프
     ├─ 스파이크 (새 서브에이전트) ──▶ 최소 프로토타입, 증거 회수
     ├─ murex commit ─────────────▶ continue | pivot | stop, 비용, 증거
     │
     └─ remaining_exposure가 0에 도달하거나 commit이 stop을 결정할 때까지 반복
```

`murex`는 결정론적 장부입니다 — 리스크 등록부, 노출 순위(확률 × 영향),
커밋먼트 게이트 — 그리고 절대 직접 일하지 않습니다. 스파이크는 지휘하는
에이전트의 새 서브에이전트에서 돕니다 — 외부 엔진 없이 같은 격리를
얻습니다. 품질 루프가 게이트를 통과할 때까지 돈다면, 이 루프는 리스크가
소진될 때까지 돕니다. 상태는 대상 저장소의 `.murex/spiral.json`에 평문
JSON으로 기록됩니다.

## 왜 애자일이 아니라 나선형인가

애자일의 짧은 스프린트와 작은 증분은 코드를 만드는 일이 느리고 비싸던
시절에 나온 장치입니다. AI 에이전트는 만드는 비용을 거의 없애버렸습니다.
이제 비싼 것은 잘못된 가정 위에 빠르게 쌓아 올리는 일입니다. 에이전트는
빠르고 자신만만해서, 방향이 틀린 코드도 테스트가 통과할 때까지 기꺼이
다듬어 냅니다. 그래서 murex는 진행을 "얼마나 만들었나"가 아니라 "위험을
얼마나 없앴나"로 셉니다. 비용은 불확실성을 줄일 때만 의미가 있고, 계속할
가치가 없다면 멈추는 것도 정당한 결론입니다.

## 설치

```bash
curl -fsSL https://raw.githubusercontent.com/janek-moon/murex/main/install.sh | sh
```

최신 릴리스에서 플랫폼에 맞는 사전 빌드 바이너리를 받습니다 — 툴체인이
필요 없습니다. 체크아웃에서 `./install.sh`를 돌리면 같은 일에 더해 Codex용
스킬 링크까지 하고, 플랫폼에 맞는 릴리스가 없으면
`cargo install --path .`로 대체합니다.

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
# 브리프를 새 서브에이전트로 실행
murex commit --decision continue --cost 1.5 --resolve R1 --evidence "RSS 380MB"
murex status                         # 반경 + 잔여 노출
```

모든 명령은 `--root <repo>`를 받습니다(기본 `.`).

두 번째 스킬(`skills/audit/SKILL.md`, 호출은 `/murex:audit`)은 진행 중인
장부를 바이너리가 강제할 수 없는 규율 기준으로 검토합니다 — 점수가 실제로
순위를 만드는지, 게이트 증거가 실제로 리스크를 해소하는지, 노출이 실제로
줄어드는지 — 그리고 나선 흉내를 낸 증분 개발을 적발합니다. 판정은
`.murex/spiral.json`의 리스크 id와 사이클 번호를 인용합니다.
