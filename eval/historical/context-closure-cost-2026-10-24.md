# Historical called-function context-closure cost — 2026-10-24

This experiment estimates the marginal context cost of following one hop of sccache function calls from the historical edited regions. The current brute-force prompt already contains the complete post-edit contents of every changed file, so definitions from changed files are excluded from the marginal counts below.

Two roots were compared:

1. **±5 lines** — collect calls appearing within five post-edit lines of every changed/deleted anchor.
2. **Touched function** — collect calls anywhere inside every Rust function touched by a changed/deleted anchor.

For each discovered call name, the analysis includes every matching Rust function definition elsewhere in the exact historical sccache revision. This is deliberately conservative: it is a Rust-aware textual function/call index, not type-resolved rust-analyzer resolution, so common method names such as `new` can pull multiple candidate definitions. The byte/token numbers are therefore useful upper bounds. Tokens use `tiktoken` `o200k_base` only as a common estimate; provider tokenizers differ.

| Fixture | ±5 extra tokens | Touched-function extra tokens | Δ | ±5 external definitions | Touched-function definitions |
| --- | ---: | ---: | ---: | ---: | ---: |
| #2910 `669e5a8` | 16,718 | 16,560 | -158 | 127 | 126 |
| #2912 `2a92bef` | 12,049 | 12,817 | +768 | 108 | 116 |
| #2912 `7360440` | 12,049 | 12,817 | +768 | 108 | 116 |
| #2914 `9420052` | 18,949 | 29,072 | +10,123 | 159 | 225 |
| #2916 `1ac9ec9` | 10,944 | 10,944 | 0 | 91 | 91 |
| **All five snapshots** | **70,709** | **82,210** | **+11,501** | **593** | **674** |

The touched-function policy is only about 16% larger over the whole corpus. The worst expansion is #2914, where it rises from about 18.9k to 29.1k estimated tokens. For the current million-token paid models that marginal input is economically negligible relative to inference latency/attention: even the 29.1k-token worst case is below half a US cent of extra input at GLM's current $0.15/M input rate, and lower for Muse/Luna/DeepSeek.

## The #2912 result is the important one

The broader root is materially better, not merely larger. On `7360440…`, ±5-line closure reaches `share_jobserver` helpers but does not necessarily include the unchanged process-launch implementation. Touched-function closure additionally reaches `src/mock_command.rs::AsyncCommand::spawn`, which shows the real child boundary:

- `MAKEFLAGS`, `MFLAGS`, and `CARGO_MAKEFLAGS` are removed by default;
- sharing the jobserver calls `jobserver.configure(&mut inner)`;
- the surrounding comment explains that this configures inherited jobserver file descriptors via `pre_exec`/`CLOEXEC` handling;
- acquiring an sccache-side token is explicitly separate from making the jobserver available to the child.

That is exactly the unchanged evidence the earlier GLM reasoning traces said they could not verify. In this fixture it costs only 768 estimated tokens more than ±5 closure.

## Context-window effect

Using the same common tokenizer only for rough comparison, the routed rule corpus + sccache overlay + current full changed-file/diff prompt is approximately 103k–179k tokens across the five fixtures before called-function closure. Adding touched-function closure keeps the historical cases roughly in the 116k–190k range. Therefore 256k-and-larger endpoints still have room, while a 65k endpoint cannot run this reviewer configuration at all and a 128k endpoint cannot cover the larger cases.

## Recommendation

Use **touched-function roots + one-hop type-resolved callees** as the first deterministic unchanged-context closure. It is a better semantic boundary than an arbitrary line radius and remains cheap on the historical corpus. A production implementation should use Rust symbol/type resolution rather than the conservative name matcher here, deduplicate already-visible definitions, cap expansion deterministically, and record exactly which definitions were added so context selection is inspectable.
