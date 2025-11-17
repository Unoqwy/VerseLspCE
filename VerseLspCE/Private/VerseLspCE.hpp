#pragma once

#define __INTELLISENSE__ 1


#include "uLang/Toolchain/ProgramBuildManager.h"


namespace Verse::LspCE {

#include "verse_lsp_rs.h"

using namespace uLang;

struct LspProjectContainer {
    CProgramBuildManager& _BuildManager;
    TSRef<CSourceProject> _Project;
};

RsSourceSpan TextRangeToSpan(STextRange Range);

} // namespace Verse::LspCE

