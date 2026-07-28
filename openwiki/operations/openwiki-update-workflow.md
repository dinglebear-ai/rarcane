---
type: Playbook
title: OpenWiki Update Workflow
description: Documents the repository's OpenWiki automation workflow, including scheduler, generation command, provider settings, and required pull request output paths.
tags:
  - openwiki
  - workflow
  - automation
  - documentation
---

# OpenWiki Update Workflow

## Purpose

The OpenWiki workflow in this repository performs periodic and on-demand documentation regeneration for code-mode documentation. It is the primary mechanism for keeping `openwiki/` current when source or operations files change.

## Workflow behavior

The workflow is defined in [`.github/workflows/openwiki-update.yml`](../../.github/workflows/openwiki-update.yml) and:

- runs on `workflow_dispatch` and a daily UTC schedule (`cron: 0 8 * * *`),
- runs on `ubuntu-latest` with Node.js 22,
- installs OpenWiki via `npm install --global openwiki`,
- executes `openwiki code --update --print`, and
- uses `OPENWIKI_PROVIDER=openrouter` with `OPENWIKI_MODEL_ID=z-ai/glm-5.2`.

## Environment and tracing

The job uses these relevant variables and secrets:

- `OPENROUTER_API_KEY` to authenticate to OpenRouter,
- `OPENWIKI_PROVIDER=openrouter`,
- `OPENWIKI_MODEL_ID=z-ai/glm-5.2`, and
- optional tracing fields (`LANGSMITH_API_KEY`, `LANGCHAIN_PROJECT`, `LANGCHAIN_TRACING_V2`) for observability.

## Pull request automation

The workflow uses `peter-evans/create-pull-request` to post generated documentation updates to the `openwiki/update` branch. The `add-paths` scope includes:

- `openwiki`
- `AGENTS.md`
- `CLAUDE.md`
- `.github/workflows/openwiki-update.yml`

## Source-level context

The workflow file is the source of truth for schedule, provider, model, command, and pull-request paths. Regenerate the linked docs after changing that workflow.

## Cross-links

- The top-level contributor entrypoint is [OpenWiki Quickstart](../quickstart.md).
