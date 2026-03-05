# Gemini OS Build Runbook

## Start best interactive mode

```bash
gemini -m gemini-2.5-pro --approval-mode auto_edit
```

## Deep architecture pass (non-interactive)

```bash
gemini -m gemini-2.5-pro -p "Use GEMINI.md context. Produce architecture + threat model + module contracts for PhantomKernel OS."
```

## Fast iteration pass

```bash
gemini -m gemini-2.5-flash -p "Refactor this section for clarity and implementation readiness."
```

## Plan-only safety review

```bash
gemini -m gemini-2.5-pro --approval-mode plan -p "Review roadmap risks and produce mitigation plan."
```

## Recommended workflow

1. Use `MASTER_PROMPT.md` first for full baseline spec.
2. Run `CODEX_PROMPT.md` for implementation details.
3. Run `KIMI_PROMPT.md` for architecture consistency checks.
4. Run `OPUS_PROMPT.md` for adversarial risk review.
5. Merge results into `PHANTOMKERNEL_OS_BIBLE.md`.
