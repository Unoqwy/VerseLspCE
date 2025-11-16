#pragma once

#define __INTELLISENSE__ 1


#include "uLang/Toolchain/ProgramBuildManager.h"


namespace Verse::LspCE
{

using namespace uLang;

struct LspProjectContainer {
    CProgramBuildManager& _BuildManager;
    TSRef<CSourceProject> _Project;
    TSPtr<CAstProject> _LastAstProject;
};

} // namespace Verse::LspCE

