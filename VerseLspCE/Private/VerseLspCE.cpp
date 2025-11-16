#include "VerseLspCE.hpp"

#include "verse_lsp_rs.h"

#include "ULangUE.h"
#include "uLang/Toolchain/ModularFeatureManager.h"
#include "uLang/Parser/ParserPass.h"
#include "uLang/SemanticAnalyzer/SemanticAnalyzerPass.h"
#include "uLang/SemanticAnalyzer/IRGeneratorPass.h"

using namespace Verse::LspCE;


/* Constants required by UE */
TCHAR GInternalProjectName[64] = TEXT("");
const TCHAR* GForeignEngineDir = nullptr;


int main(int ArgC, char* ArgV[]) {
    uLangUE::Initialize(); // (allocator)

    // Modular features definitions. RAII pattern so they are initialized/registered here.
    TModularFeatureRegHandle<CParserPass> ParserPassFeature;
    TModularFeatureRegHandle<CSemanticAnalyzerPass> SemanticAnalyzerPassFeature;
    TModularFeatureRegHandle<CIrGeneratorPass> IRGeneratorPassFeature;

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

    // FIXME: This _AstProject field on BuildResult is patched into UE source code.
    //        I would like to do without it to avoid using a custom fork, but there is a strange memory issue I can't figure out.
    //        All that is added is `BuildResult._AstProject = ProgramContext._Program->_AstProject` at the very end of BuildProject.
    //        Trying to do the same thing outside BuildProject suddenly doesn't work because the TSPtr itself has a corrupted memory address.
    //        (ProgramContext._Program is valid and pointing to the same object)
    //        The odd part is, ~CSemanticProgram (destructor) sees the correct TSPtr address, but not any other call.
    //        This patch is a hack after I spent way too much time trying to figure out the true issue...
    //        Welcome to the madness. Grab a drink!
    //        This shit wouldn't happen if VerseCompiler was written in Rust, just saying.
    //        Note: Still safe to keep in memory despite everything.
    ProjectContainer->_LastAstProject = BuildResult._AstProject;

    for (const auto& Glitch : Diagnostics->GetGlitches()) {
        auto Range = Glitch->_Locus._Range;
        auto GlitchInfo = Glitch->_Result.GetInfo();
        RsDiagnostic Diagnostic = {
            ._Path = Glitch->_Locus._SnippetPath.AsCString(),
            ._Message = Glitch->_Result._Message.AsCString(),
            ._ReferenceCode = GlitchInfo.ReferenceCode,
            ._BeginRow = Range.BeginRow(),
            ._BeginColumn = Range.BeginColumn(),
            ._EndRow = Range.EndRow(),
            ._EndColumn = Range.EndColumn(),
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

