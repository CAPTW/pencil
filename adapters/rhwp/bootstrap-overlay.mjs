// Opt-in custom Studio build seam. Default policy is denial, never URL authority.
export function overlayBootstrap(source,{syntheticFixture=false}={}) {
  const replacements=[
    ["import { DocumentAgentController } from '@/document-agent/controller';",
     "import { DocumentAgentController, collectTargetEvidence } from '@/document-agent/controller';\nimport { installReview } from '/grammar-review/bootstrap.mjs';"],
    ['let documentAgent: DocumentAgentController | null = null;',
     'let documentAgent: DocumentAgentController | null = null;\nlet grammarReviewDispose: (() => void) | null = null;'],
    ['    documentAgent?.dispose();','    grammarReviewDispose?.();\n    documentAgent?.dispose();'],
    ['      render: () => canvasView!.refreshDocumentAgentMutation(),\n    });',
     `      render: () => canvasView!.refreshDocumentAgentMutation(),
    });
    let grammarHost = document.getElementById('grammar-review-host');
    if (!grammarHost) {
      grammarHost = document.createElement('aside');grammarHost.id='grammar-review-host';
      grammarHost.setAttribute('aria-label','Grammar review');
      grammarHost.style.cssText='position:fixed;right:12px;top:70px;z-index:10000;background:white;color:black;padding:12px;width:320px;border:1px solid #777';
      document.body.append(grammarHost);
    }
    grammarReviewDispose=installReview({host:grammarHost,Controller:DocumentAgentController,collectEvidence:collectTargetEvidence,
      deps:{wasm,input:inputHandler,eventBus,isDirty:()=>documentState.isDirty(),render:()=>canvasView!.refreshDocumentAgentMutation()},
      eligible:()=>${syntheticFixture?"editMode === 'normal' && ['hwp','hwpx'].includes(wasm.getSourceFormat())":'false'}});
`]
  ];
  for(const [before,after] of replacements){if(source.split(before).length!==2)throw new Error('RHWP_BOOTSTRAP_SOURCE_DRIFT');source=source.replace(before,after);}
  return source;
}
