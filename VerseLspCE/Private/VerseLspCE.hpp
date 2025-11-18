#pragma once

#define __INTELLISENSE__ 1


#include "uLang/Toolchain/ProgramBuildManager.h"


namespace Verse::LspCE {

#include "verse_lsp_rs.h"

using namespace uLang;

struct LspProjectContainer {
    TSRef<CSourceProject> _Project;

    CProgramBuildManager& _BuildManager;
    SProgramContext* _ProgramContext;
    TSPtr<CSymbolTable> _Symbols;
};

RsSourceSpan TextRangeToSpan(STextRange Range);

} // namespace Verse::LspCE

