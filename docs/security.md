# RetinaCare security boundaries

- New native app stores clinical records only in local SQLCipher-backed storage; do not use IndexedDB/browser persistence.
- Keep database keys and passwords out of logs, arguments, manifests, source, and Git.
- The UI calls domain repositories; never expose arbitrary SQL execution commands.
- Keep worker communication on local stdin/stdout JSONL. No HTTP listener or remote inference endpoint.
- Validate message version, request IDs, image paths, file types, and checkpoint manifests; bound message and image sizes.
- Do not log patient names, IDs, image content, or model output containing identifiers.
- Keep `.dird` imports transactional and validate all archive members before committing records.
- Preserve legacy security boundaries until new storage and import paths have recovery tests.
