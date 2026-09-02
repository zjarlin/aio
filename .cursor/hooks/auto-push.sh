#!/bin/bash
set -u

# stop 钩子只处理当前工作区，避免替用户覆盖无法判断语义的冲突。
input=$(cat)
repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
  printf '%s\n' '{"followup_message":"当前目录不是 Git 仓库，未执行自动推送。"}'
  exit 0
}
cd "$repo_root" || exit 0

branch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null) || {
  printf '%s\n' '{"followup_message":"当前处于 detached HEAD，未执行自动推送。"}'
  exit 0
}
remote=$(git config "branch.${branch}.remote" 2>/dev/null || true)
merge_ref=$(git config "branch.${branch}.merge" 2>/dev/null || true)

if [[ -z "$remote" || -z "$merge_ref" || "$remote" == "." ]]; then
  printf '%s\n' "{\"followup_message\":\"分支 ${branch} 没有配置远程跟踪分支，未执行自动推送。\"}"
  exit 0
fi

if [[ -n "$(git status --porcelain=v1 --untracked-files=all)" ]]; then
  git add -A
  if ! git diff --cached --quiet; then
    commit_message="自动保存：$(date '+%Y-%m-%d %H:%M:%S %z')"
    if ! git commit -m "$commit_message" >/tmp/cursor-stop-git-commit.log 2>&1; then
      printf '%s\n' '{"followup_message":"自动提交失败，已停止推送。请查看 Git 状态和提交日志。"}'
      exit 0
    fi
  fi
fi

# 先获取远程提交，再用 rebase 合并；Git 能自动处理的变更会直接完成。
if ! git fetch "$remote" >/tmp/cursor-stop-git-fetch.log 2>&1; then
  printf '%s\n' '{"followup_message":"获取远程代码失败，已保留本地提交，未执行推送。"}'
  exit 0
fi

upstream="${remote}/${merge_ref#refs/heads/}"
if ! git rebase "$upstream" >/tmp/cursor-stop-git-rebase.log 2>&1; then
  git rebase --abort >/dev/null 2>&1 || true
  printf '%s\n' '{"followup_message":"自动同步遇到需要人工判断的冲突，已中止 rebase，未执行推送。请解决冲突并重新运行。"}'
  exit 0
fi

if ! git push "$remote" "$branch" >/tmp/cursor-stop-git-push.log 2>&1; then
  printf '%s\n' '{"followup_message":"自动推送失败。请检查远程权限、网络和 Git 状态。"}'
  exit 0
fi

printf '%s\n' '{"followup_message":"当前会话结束前已自动提交、同步并推送工作区代码。"}'
exit 0
