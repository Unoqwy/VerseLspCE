#include "VerseLspCE.hpp"

#include "ULangUE.h"
#include "uLang/Toolchain/ModularFeatureManager.h"
#include "uLang/Parser/ParserPass.h"
#include "uLang/SemanticAnalyzer/SemanticAnalyzerPass.h"

using namespace Verse::LspCE;


/* Constants required by UE */
TCHAR GInternalProjectName[64] = TEXT("");
const TCHAR* GForeignEngineDir = nullptr;


int main(int ArgC, char* ArgV[]) {
    uLangUE::Initialize(); // (allocator)

    // Modular features definitions. RAII pattern so they are registered here and need to held.
    TModularFeatureRegHandle<CParserPass> ParserPassFeature;
    TModularFeatureRegHandle<CSemanticAnalyzerPass> SemanticAnalyzerPassFeature;

    RS_RunServer();

    return 0;
}

extern "C" LspProjectContainer* Lsp_RegisterProjectContainer(
    const char* ProjectName
) {
    SBuildManagerParams ManagerParams;
    CProgramBuildManager* BuildManager = new CProgramBuildManager(ManagerParams);

    TSRef<CSourceProject> Project = TSRef<CSourceProject>::New(CUTF8String(ProjectName));
    BuildManager->SetSourceProject(Project);

    LspProjectContainer* ProjectContainer = new LspProjectContainer {
        ._BuildManager = *BuildManager,
        ._Project = Project,
    };

    // NOTE: Arbitrary limit of packages, that if exceeded would cause package pointers to be fucked by regrowth
    //       In practice the number of packages with a vproject should always be below that number for now
    // TODO: A better way to handle Rust references to packages (maybe Project* + index?)
    ProjectContainer->_Project->_Packages.Reserve(16);

    return ProjectContainer;
}


extern "C" void Lsp_Build(
    LspProjectContainer* ProjectContainer,
    RsDiagnosticAccumulator* DiagnosticAccumulator
) {
    const auto Diagnostics = TSRef<CDiagnostics>::New();

    SBuildParams BuildParams = {
        ._MaxNumPersistentVars = 2,
        ._MaxNumConcreteProductSubclasses = 100,
        ._bSemanticAnalysisOnly = true,
        ._bGenerateDigests = false,
        ._bGenerateCode = false,
    };

    SBuildContext BuildContext(Diagnostics);
    BuildContext._Params = BuildParams;
    BuildContext.bCloneValidSnippetVsts = true;

    CProgramBuildManager& BuildManager = ProjectContainer->_BuildManager;

    BuildManager.ResetSemanticProgram();
    SBuildResults BuildResult = BuildManager.GetToolchain()->BuildProject(
            *BuildManager.GetSourceProject(), BuildContext, BuildManager.GetProgramContext());

    for (const auto& Glitch : Diagnostics->GetGlitches()) {
        auto Range = Glitch->_Locus._Range;
        auto GlitchInfo = Glitch->_Result.GetInfo();
        RsDiagnostic Diagnostic = {
            ._Path = Glitch->_Locus._SnippetPath.AsCString(),
            ._Message = Glitch->_Result._Message.AsCString(),
            ._ReferenceCode = GlitchInfo.ReferenceCode,
            ._Span = TextRangeToSpan(Range),
        };

        int32_t SeverityCode = 0;
        switch (GlitchInfo.Severity) {
            case EDiagnosticSeverity::Error: SeverityCode = 1; break;
            case EDiagnosticSeverity::Warning: SeverityCode = 2; break;
            case EDiagnosticSeverity::Info: SeverityCode = 3; break;
            case EDiagnosticSeverity::Ok: break;
            default: break;
        };
        Diagnostic._Severity = SeverityCode;

        // AddDiagnostic creates owned Rust strings from char* pointers
        RS_AddDiagnostic(DiagnosticAccumulator, Diagnostic);
    }
}

namespace Verse::LspCE
{

RsSourceSpan TextRangeToSpan(STextRange Range) {
    return {
        ._BeginRow = Range.BeginRow(),
        ._BeginColumn = Range.BeginColumn(),
        ._EndRow = Range.EndRow(),
        ._EndColumn = Range.EndColumn(),
    };
}

} // namespace Verse::LspCE

