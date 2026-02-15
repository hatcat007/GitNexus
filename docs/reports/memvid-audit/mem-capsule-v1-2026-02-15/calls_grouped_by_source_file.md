# Calls Grouped By Source File

- groups: 87

## 1. backend/src/gateway/server.ts
- calls: 140
- sourceLabels: File, Function
- sourceFunctions: 25
- calledSymbols: 99
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/backend/src/gateway/server.ts:createStorageBundle->Function:MEM-Capsule v1/backend/src/storage/factory.ts:createStorageBundle
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/gateway/server.ts:parseInlineUploadFiles:asRecord->Function:MEM-Capsule v1/backend/src/gateway/server.ts:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/gateway/server.ts:parseInlineUploadFiles:pickString->Function:MEM-Capsule v1/backend/src/gateway/server.ts:pickString

## 2. backend/src/worker/verification/service.ts
- calls: 85
- sourceLabels: Function
- sourceFunctions: 21
- calledSymbols: 38
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/service.ts:normalizeScope:clamp->Function:MEM-Capsule v1/backend/src/worker/verification/service.ts:clamp
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/service.ts:sanitizeVerificationSources:asRecord->Function:MEM-Capsule v1/backend/src/worker/verification/service.ts:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/service.ts:sanitizeVerificationSources:trimToMax->Function:MEM-Capsule v1/backend/src/worker/verification/service.ts:trimToMax

## 3. backend/src/worker/server.ts
- calls: 72
- sourceLabels: File, Function
- sourceFunctions: 11
- calledSymbols: 57
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/server.ts:requireCapsulePath:asRecord->Function:MEM-Capsule v1/backend/src/worker/server.ts:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/server.ts:openExistingCapsuleOrThrow:loadMemvid->Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:loadMemvid
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/server.ts:openExistingCapsuleOrThrow:openExistingCapsule->Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:openExistingCapsule

## 4. backend/src/worker/memvid-adapter.ts
- calls: 56
- sourceLabels: Function
- sourceFunctions: 25
- calledSymbols: 25
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:asRecord:isRecord->Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:isRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:toIntValue:toNumberValue->Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:toNumberValue
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:toUnixTimestamp:toNumberValue->Function:MEM-Capsule v1/backend/src/worker/memvid-adapter.ts:toNumberValue

## 5. src/lib/api.ts
- calls: 52
- sourceLabels: File, Function
- sourceFunctions: 39
- calledSymbols: 15
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/lib/api.ts:normalizeApiBase->Function:MEM-Capsule v1/src/lib/api.ts:normalizeApiBase
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/api.ts:toNotice:asRecord->Function:MEM-Capsule v1/src/lib/api.ts:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/api.ts:toNotice:isNoticeLevel->Function:MEM-Capsule v1/src/lib/api.ts:isNoticeLevel

## 6. backend/src/worker/graph/pipeline.ts
- calls: 44
- sourceLabels: Function
- sourceFunctions: 17
- calledSymbols: 30
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/pipeline.ts:stripTrailingMetadata:clean->Function:MEM-Capsule v1/backend/src/worker/graph/pipeline.ts:clean
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/pipeline.ts:normalizeGroup:clean->Function:MEM-Capsule v1/backend/src/worker/graph/pipeline.ts:clean
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/pipeline.ts:parseImports:clean->Function:MEM-Capsule v1/backend/src/worker/graph/pipeline.ts:clean

## 7. src/pages/MemoryDetail.tsx
- calls: 41
- sourceLabels: Function
- sourceFunctions: 12
- calledSymbols: 34
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/MemoryDetail.tsx:formatRelativeRelevance:normalizeScore->Function:MEM-Capsule v1/src/pages/MemoryDetail.tsx:normalizeScore
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/MemoryDetail.tsx:MemoryDetail:useToast->Function:MEM-Capsule v1/src/hooks/use-toast.ts:useToast
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/MemoryDetail.tsx:MemoryDetail:loadAppSettings->Function:MEM-Capsule v1/src/lib/app-settings.ts:loadAppSettings

## 8. backend/src/worker/graph/analysis.ts
- calls: 32
- sourceLabels: Function
- sourceFunctions: 14
- calledSymbols: 15
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/analysis.ts:buildIndex:get->Method:MEM-Capsule v1/backend/src/stores/capsule-store.ts:get
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/analysis.ts:pickSubgraph:computeMetrics->Function:MEM-Capsule v1/backend/src/worker/graph/analysis.ts:computeMetrics
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/analysis.ts:nodeMatchesQuery:normalizeText->Function:MEM-Capsule v1/backend/src/worker/graph/analysis.ts:normalizeText

## 9. backend/src/services/worker-client.ts
- calls: 27
- sourceLabels: Method
- sourceFunctions: 26
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/services/worker-client.ts:post:redact->Function:MEM-Capsule v1/backend/src/utils/redact.ts:redact
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/services/worker-client.ts:post:parseWorkerMessage->Function:MEM-Capsule v1/backend/src/services/worker-client.ts:parseWorkerMessage
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/services/worker-client.ts:health:post->Method:MEM-Capsule v1/backend/src/services/worker-client.ts:post

## 10. backend/src/worker/openrouter.ts
- calls: 26
- sourceLabels: Function
- sourceFunctions: 11
- calledSymbols: 20
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/openrouter.ts:extractJson:parseCandidate->Function:MEM-Capsule v1/backend/src/worker/openrouter.ts:parseCandidate
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/openrouter.ts:extractJson:stripFence->Function:MEM-Capsule v1/backend/src/worker/openrouter.ts:stripFence
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/openrouter.ts:extractJson:sliceBalanced->Function:MEM-Capsule v1/backend/src/worker/openrouter.ts:sliceBalanced

## 11. src/pages/UploadPage.tsx
- calls: 25
- sourceLabels: Function
- sourceFunctions: 5
- calledSymbols: 19
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/UploadPage.tsx:UploadPage:loadAppSettings->Function:MEM-Capsule v1/src/lib/app-settings.ts:loadAppSettings
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/UploadPage.tsx:UploadPage:useToast->Function:MEM-Capsule v1/src/hooks/use-toast.ts:useToast
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/UploadPage.tsx:UploadPage:getFileType->Function:MEM-Capsule v1/src/pages/UploadPage.tsx:getFileType

## 12. backend/src/storage/redis-metadata-store.ts
- calls: 23
- sourceLabels: Method
- sourceFunctions: 7
- calledSymbols: 6
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:list:indexKey->Method:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:indexKey
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:list:get->Method:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:get
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:list:key->Method:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:key

## 13. backend/src/worker/graph/modes.ts
- calls: 21
- sourceLabels: Function
- sourceFunctions: 7
- calledSymbols: 15
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/modes.ts:isContext7Node:lower->Function:MEM-Capsule v1/backend/src/worker/graph/modes.ts:lower
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/modes.ts:isContext7Node:asRecord->Function:MEM-Capsule v1/backend/src/worker/graph/modes.ts:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/modes.ts:modeNodeBoost:isContext7Node->Function:MEM-Capsule v1/backend/src/worker/graph/modes.ts:isContext7Node

## 14. backend/src/services/job-queue.ts
- calls: 19
- sourceLabels: Method
- sourceFunctions: 5
- calledSymbols: 14
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/services/job-queue.ts:enqueue:tick->Method:MEM-Capsule v1/backend/src/services/job-queue.ts:tick
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/services/job-queue.ts:runImmediately:runJob->Method:MEM-Capsule v1/backend/src/services/job-queue.ts:runJob
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/services/job-queue.ts:tick:runJob->Method:MEM-Capsule v1/backend/src/services/job-queue.ts:runJob

## 15. backend/src/worker/graph.ts
- calls: 14
- sourceLabels: Function
- sourceFunctions: 6
- calledSymbols: 10
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph.ts:stripTrailingMetadata:clean->Function:MEM-Capsule v1/backend/src/worker/graph.ts:clean
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph.ts:normalizeGroup:clean->Function:MEM-Capsule v1/backend/src/worker/graph.ts:clean
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph.ts:uniqueMatches:clean->Function:MEM-Capsule v1/backend/src/worker/graph.ts:clean

## 16. backend/src/worker/verification/providers/grokipedia.ts
- calls: 12
- sourceLabels: Function
- sourceFunctions: 4
- calledSymbols: 9
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/providers/grokipedia.ts:buildSlugCandidates:toSlugCandidate->Function:MEM-Capsule v1/backend/src/worker/verification/providers/grokipedia.ts:toSlugCandidate
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/providers/grokipedia.ts:collectPageSources:pickString->Function:MEM-Capsule v1/backend/src/worker/verification/providers/grokipedia.ts:pickString
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/providers/grokipedia.ts:collectPageSources:clip->Function:MEM-Capsule v1/backend/src/worker/verification/providers/grokipedia.ts:clip

## 17. src/components/KnowledgeGraph.tsx
- calls: 11
- sourceLabels: Function
- sourceFunctions: 2
- calledSymbols: 11
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/KnowledgeGraph.tsx:toGraphData:asRecord->Function:MEM-Capsule v1/src/components/KnowledgeGraph.tsx:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/KnowledgeGraph.tsx:toGraphData:clamp->Function:MEM-Capsule v1/src/components/KnowledgeGraph.tsx:clamp
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/KnowledgeGraph.tsx:toGraphData:get->Method:MEM-Capsule v1/backend/src/stores/capsule-store.ts:get

## 18. src/test/api.test.ts
- calls: 11
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 11
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/test/api.test.ts:okJson->Function:MEM-Capsule v1/src/test/api.test.ts:okJson
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/test/api.test.ts:ingestCapsule->Function:MEM-Capsule v1/src/lib/api.ts:ingestCapsule
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/test/api.test.ts:listCapsules->Function:MEM-Capsule v1/src/lib/api.ts:listCapsules

## 19. src/hooks/use-toast.ts
- calls: 10
- sourceLabels: Function
- sourceFunctions: 7
- calledSymbols: 6
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/hooks/use-toast.ts:addToRemoveQueue:delete->Method:MEM-Capsule v1/backend/src/stores/capsule-store.ts:delete
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/hooks/use-toast.ts:addToRemoveQueue:dispatch->Function:MEM-Capsule v1/src/hooks/use-toast.ts:dispatch
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/hooks/use-toast.ts:reducer:addToRemoveQueue->Function:MEM-Capsule v1/src/hooks/use-toast.ts:addToRemoveQueue

## 20. src/pages/Dashboard.tsx
- calls: 10
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 9
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/Dashboard.tsx:Dashboard:useToast->Function:MEM-Capsule v1/src/hooks/use-toast.ts:useToast
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/Dashboard.tsx:refresh:listCapsules->Function:MEM-Capsule v1/src/lib/api.ts:listCapsules
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/Dashboard.tsx:Dashboard:refresh->Function:MEM-Capsule v1/src/pages/Dashboard.tsx:refresh

## 21. backend/src/storage/blob-binary-store.ts
- calls: 9
- sourceLabels: Method
- sourceFunctions: 5
- calledSymbols: 7
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/blob-binary-store.ts:materializeCapsule:resolveBlobUrl->Method:MEM-Capsule v1/backend/src/storage/blob-binary-store.ts:resolveBlobUrl
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/blob-binary-store.ts:materializeCapsule:localCachePath->Function:MEM-Capsule v1/backend/src/storage/blob-binary-store.ts:localCachePath
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/blob-binary-store.ts:materializeCapsule:fetchBlobBytes->Function:MEM-Capsule v1/backend/src/storage/blob-binary-store.ts:fetchBlobBytes

## 22. backend/src/worker/verification/providers/jina.ts
- calls: 9
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 5
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/providers/jina.ts:collectSources:asRecord->Function:MEM-Capsule v1/backend/src/worker/verification/providers/jina.ts:asRecord
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/providers/jina.ts:collectSources:pickString->Function:MEM-Capsule v1/backend/src/worker/verification/providers/jina.ts:pickString
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/providers/jina.ts:normalizeStatus:asRecord->Function:MEM-Capsule v1/backend/src/worker/verification/providers/jina.ts:asRecord

## 23. src/pages/SearchPage.tsx
- calls: 9
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 8
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/SearchPage.tsx:relativeRelevance:normalizeScore->Function:MEM-Capsule v1/src/pages/SearchPage.tsx:normalizeScore
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/SearchPage.tsx:SearchPage:get->Method:MEM-Capsule v1/backend/src/stores/capsule-store.ts:get
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/SearchPage.tsx:SearchPage:listCapsules->Function:MEM-Capsule v1/src/lib/api.ts:listCapsules

## 24. src/lib/app-settings.ts
- calls: 8
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 7
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/app-settings.ts:sanitizeSettings:parseAttempts->Function:MEM-Capsule v1/src/lib/app-settings.ts:parseAttempts
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/app-settings.ts:sanitizeSettings:parseBackoffSeconds->Function:MEM-Capsule v1/src/lib/app-settings.ts:parseBackoffSeconds
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/app-settings.ts:sanitizeSettings:parseRelations->Function:MEM-Capsule v1/src/lib/app-settings.ts:parseRelations

## 25. src/pages/SettingsPage.tsx
- calls: 8
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 7
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/SettingsPage.tsx:SettingsPage:loadAppSettings->Function:MEM-Capsule v1/src/lib/app-settings.ts:loadAppSettings
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/SettingsPage.tsx:SettingsPage:useToast->Function:MEM-Capsule v1/src/hooks/use-toast.ts:useToast
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/SettingsPage.tsx:handleSave:saveAppSettings->Function:MEM-Capsule v1/src/lib/app-settings.ts:saveAppSettings

## 26. src/components/ui/pagination.tsx
- calls: 6
- sourceLabels: File, Function
- sourceFunctions: 6
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/pagination.tsx:Pagination:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/pagination.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/pagination.tsx:PaginationLink:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 27. backend/src/worker/graph/store.ts
- calls: 5
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/store.ts:saveGraphArtifact:ensureDir->Function:MEM-Capsule v1/backend/src/utils/fs.ts:ensureDir
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/store.ts:saveGraphArtifact:graphPathFor->Function:MEM-Capsule v1/backend/src/worker/graph/store.ts:graphPathFor
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/graph/store.ts:loadGraphArtifact:graphPathFor->Function:MEM-Capsule v1/backend/src/worker/graph/store.ts:graphPathFor

## 28. backend/src/config.ts
- calls: 4
- sourceLabels: File, Function
- sourceFunctions: 2
- calledSymbols: 4
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/config.ts:resolveDefaultStorageRoot:toMemCapsuleRoot->Function:MEM-Capsule v1/backend/src/config.ts:toMemCapsuleRoot
  - mv2://relations/CALLS:File:MEM-Capsule v1/backend/src/config.ts:detectContainerRuntime->Function:MEM-Capsule v1/backend/src/config.ts:detectContainerRuntime
  - mv2://relations/CALLS:File:MEM-Capsule v1/backend/src/config.ts:resolveDefaultStorageRoot->Function:MEM-Capsule v1/backend/src/config.ts:resolveDefaultStorageRoot

## 29. backend/src/storage/fs-binary-store.ts
- calls: 4
- sourceLabels: Method
- sourceFunctions: 3
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/fs-binary-store.ts:persistCapsule:statSize->Function:MEM-Capsule v1/backend/src/utils/fs.ts:statSize
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/fs-binary-store.ts:persistArtifact:artifactPathFor->Function:MEM-Capsule v1/backend/src/storage/fs-binary-store.ts:artifactPathFor
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/storage/fs-binary-store.ts:persistArtifact:ensureDir->Function:MEM-Capsule v1/backend/src/utils/fs.ts:ensureDir

## 30. backend/src/worker/verification/scorer.ts
- calls: 4
- sourceLabels: Function
- sourceFunctions: 2
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/scorer.ts:tokenizeEntity:normalize->Function:MEM-Capsule v1/backend/src/worker/verification/scorer.ts:normalize
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/scorer.ts:scoreEvidence:tokenizeEntity->Function:MEM-Capsule v1/backend/src/worker/verification/scorer.ts:tokenizeEntity
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/worker/verification/scorer.ts:scoreEvidence:normalize->Function:MEM-Capsule v1/backend/src/worker/verification/scorer.ts:normalize

## 31. src/pages/GlobalGraph.tsx
- calls: 4
- sourceLabels: Function
- sourceFunctions: 3
- calledSymbols: 4
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/GlobalGraph.tsx:GlobalGraphPage:getGlobalGraph->Function:MEM-Capsule v1/src/lib/api.ts:getGlobalGraph
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/GlobalGraph.tsx:handleSearch:searchGlobalGraph->Function:MEM-Capsule v1/src/lib/api.ts:searchGlobalGraph
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/pages/GlobalGraph.tsx:handleImpact:impactGlobalGraph->Function:MEM-Capsule v1/src/lib/api.ts:impactGlobalGraph

## 32. backend/src/stores/job-store.ts
- calls: 3
- sourceLabels: Method
- sourceFunctions: 3
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/stores/job-store.ts:write:writeJsonFile->Function:MEM-Capsule v1/backend/src/utils/fs.ts:writeJsonFile
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/stores/job-store.ts:update:nowIso->Function:MEM-Capsule v1/backend/src/config.ts:nowIso
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/stores/job-store.ts:appendLog:get->Method:MEM-Capsule v1/backend/src/stores/job-store.ts:get

## 33. src/components/ApiToastBridge.tsx
- calls: 3
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ApiToastBridge.tsx:ApiToastBridge:subscribeApiNotices->Function:MEM-Capsule v1/src/lib/api.ts:subscribeApiNotices
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ApiToastBridge.tsx:ApiToastBridge:toast->Function:MEM-Capsule v1/src/hooks/use-toast.ts:toast
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ApiToastBridge.tsx:ApiToastBridge:titleForNotice->Function:MEM-Capsule v1/src/components/ApiToastBridge.tsx:titleForNotice

## 34. src/components/Layout.tsx
- calls: 3
- sourceLabels: Function
- sourceFunctions: 2
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/Layout.tsx:Layout:getStoredTheme->Function:MEM-Capsule v1/src/lib/theme.ts:getStoredTheme
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/Layout.tsx:Layout:applyTheme->Function:MEM-Capsule v1/src/lib/theme.ts:applyTheme
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/Layout.tsx:toggleTheme:setTheme->Function:MEM-Capsule v1/src/lib/theme.ts:setTheme

## 35. src/components/ui/alert-dialog.tsx
- calls: 3
- sourceLabels: File, Function
- sourceFunctions: 3
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/alert-dialog.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/alert-dialog.tsx:AlertDialogHeader:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/alert-dialog.tsx:AlertDialogFooter:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 36. src/components/ui/breadcrumb.tsx
- calls: 3
- sourceLabels: File, Function
- sourceFunctions: 3
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/breadcrumb.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/breadcrumb.tsx:BreadcrumbSeparator:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/breadcrumb.tsx:BreadcrumbEllipsis:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 37. src/components/ui/chart.tsx
- calls: 3
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/chart.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/chart.tsx:useChart->Function:MEM-Capsule v1/src/components/ui/chart.tsx:useChart
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/chart.tsx:getPayloadConfigFromPayload->Function:MEM-Capsule v1/src/components/ui/chart.tsx:getPayloadConfigFromPayload

## 38. src/components/ui/dialog.tsx
- calls: 3
- sourceLabels: File, Function
- sourceFunctions: 3
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/dialog.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/dialog.tsx:DialogHeader:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/dialog.tsx:DialogFooter:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 39. src/components/ui/drawer.tsx
- calls: 3
- sourceLabels: File, Function
- sourceFunctions: 3
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/drawer.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/drawer.tsx:DrawerHeader:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/drawer.tsx:DrawerFooter:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 40. src/components/ui/sheet.tsx
- calls: 3
- sourceLabels: File, Function
- sourceFunctions: 3
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/sheet.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/sheet.tsx:SheetHeader:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/sheet.tsx:SheetFooter:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 41. src/components/ui/sidebar.tsx
- calls: 3
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 3
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/sidebar.tsx:useIsMobile->Function:MEM-Capsule v1/src/hooks/use-mobile.tsx:useIsMobile
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/sidebar.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/sidebar.tsx:useSidebar->Function:MEM-Capsule v1/src/components/ui/sidebar.tsx:useSidebar

## 42. backend/src/storage/factory.ts
- calls: 2
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 2
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/storage/factory.ts:createStorageBundle:createRedisMetadataStore->Function:MEM-Capsule v1/backend/src/storage/redis-metadata-store.ts:createRedisMetadataStore
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/storage/factory.ts:createStorageBundle:createFsMetadataStore->Function:MEM-Capsule v1/backend/src/storage/fs-metadata-store.ts:createFsMetadataStore

## 43. backend/src/stores/capsule-store.ts
- calls: 2
- sourceLabels: Method
- sourceFunctions: 2
- calledSymbols: 2
- sampleUris:
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/stores/capsule-store.ts:write:writeJsonFile->Function:MEM-Capsule v1/backend/src/utils/fs.ts:writeJsonFile
  - mv2://relations/CALLS:Method:MEM-Capsule v1/backend/src/stores/capsule-store.ts:update:nowIso->Function:MEM-Capsule v1/backend/src/config.ts:nowIso

## 44. src/components/ui/carousel.tsx
- calls: 2
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 2
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/carousel.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/carousel.tsx:useCarousel->Function:MEM-Capsule v1/src/components/ui/carousel.tsx:useCarousel

## 45. src/components/ui/command.tsx
- calls: 2
- sourceLabels: File, Function
- sourceFunctions: 2
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/command.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/command.tsx:CommandShortcut:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 46. src/components/ui/context-menu.tsx
- calls: 2
- sourceLabels: File, Function
- sourceFunctions: 2
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/context-menu.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/context-menu.tsx:ContextMenuShortcut:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 47. src/components/ui/dropdown-menu.tsx
- calls: 2
- sourceLabels: File, Function
- sourceFunctions: 2
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/dropdown-menu.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/dropdown-menu.tsx:DropdownMenuShortcut:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 48. src/components/ui/form.tsx
- calls: 2
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 2
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/form.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/form.tsx:useFormField->Function:MEM-Capsule v1/src/components/ui/form.tsx:useFormField

## 49. src/components/ui/menubar.tsx
- calls: 2
- sourceLabels: File, Function
- sourceFunctions: 2
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/menubar.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/menubar.tsx:MenubarShortcut:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 50. src/components/ui/resizable.tsx
- calls: 2
- sourceLabels: Function
- sourceFunctions: 2
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/resizable.tsx:ResizablePanelGroup:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/resizable.tsx:ResizableHandle:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 51. src/lib/theme.ts
- calls: 2
- sourceLabels: Function
- sourceFunctions: 2
- calledSymbols: 2
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/theme.ts:getStoredTheme:sanitizeTheme->Function:MEM-Capsule v1/src/lib/theme.ts:sanitizeTheme
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/lib/theme.ts:setTheme:applyTheme->Function:MEM-Capsule v1/src/lib/theme.ts:applyTheme

## 52. backend/api/[...route].ts
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/api/[...route].ts:handler:initializeGateway->Function:MEM-Capsule v1/backend/src/gateway/server.ts:initializeGateway

## 53. backend/api/capsules/import.ts
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/api/capsules/import.ts:handler:initializeGateway->Function:MEM-Capsule v1/backend/src/gateway/server.ts:initializeGateway

## 54. backend/api/worker/[...route].ts
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/api/worker/[...route].ts:handler:rewriteWorkerPath->Function:MEM-Capsule v1/backend/api/worker/[...route].ts:rewriteWorkerPath

## 55. backend/src/utils/fs.ts
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/utils/fs.ts:writeJsonFile:ensureDir->Function:MEM-Capsule v1/backend/src/utils/fs.ts:ensureDir

## 56. backend/src/utils/redact.ts
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/backend/src/utils/redact.ts:redactObject:redact->Function:MEM-Capsule v1/backend/src/utils/redact.ts:redact

## 57. src/components/NavLink.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/NavLink.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 58. src/components/ui/accordion.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/accordion.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 59. src/components/ui/alert.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/alert.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 60. src/components/ui/avatar.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/avatar.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 61. src/components/ui/badge.tsx
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/badge.tsx:Badge:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 62. src/components/ui/button.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/button.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 63. src/components/ui/calendar.tsx
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/calendar.tsx:Calendar:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 64. src/components/ui/card.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/card.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 65. src/components/ui/checkbox.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/checkbox.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 66. src/components/ui/hover-card.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/hover-card.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 67. src/components/ui/input-otp.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/input-otp.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 68. src/components/ui/input.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/input.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 69. src/components/ui/label.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/label.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 70. src/components/ui/navigation-menu.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/navigation-menu.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 71. src/components/ui/popover.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/popover.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 72. src/components/ui/progress.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/progress.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 73. src/components/ui/radio-group.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/radio-group.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 74. src/components/ui/scroll-area.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/scroll-area.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 75. src/components/ui/select.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/select.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 76. src/components/ui/separator.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/separator.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 77. src/components/ui/skeleton.tsx
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/skeleton.tsx:Skeleton:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 78. src/components/ui/slider.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/slider.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 79. src/components/ui/switch.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/switch.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 80. src/components/ui/table.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/table.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 81. src/components/ui/tabs.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/tabs.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 82. src/components/ui/textarea.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/textarea.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 83. src/components/ui/toast.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/toast.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 84. src/components/ui/toaster.tsx
- calls: 1
- sourceLabels: Function
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:Function:MEM-Capsule v1/src/components/ui/toaster.tsx:Toaster:useToast->Function:MEM-Capsule v1/src/hooks/use-toast.ts:useToast

## 85. src/components/ui/toggle-group.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/toggle-group.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 86. src/components/ui/toggle.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/toggle.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn

## 87. src/components/ui/tooltip.tsx
- calls: 1
- sourceLabels: File
- sourceFunctions: 1
- calledSymbols: 1
- sampleUris:
  - mv2://relations/CALLS:File:MEM-Capsule v1/src/components/ui/tooltip.tsx:cn->Function:MEM-Capsule v1/src/lib/utils.ts:cn
