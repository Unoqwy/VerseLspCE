#include "VerseLspCE.hpp"

#include "uLang/Common/Text/FilePathUtils.h"
#include "uLang/Common/Text/UTF8String.h"
#include "uLang/SourceProject/PackageRole.h"
#include "uLang/SourceProject/SourceFileProject.h"
#include "uLang/SourceProject/SourceProject.h"
#include "uLang/SourceProject/SourceDataProject.h"

#include <cstdio>

using namespace Verse::LspCE;


const CSourceProject::SPackage& RegisterPackage(
    const TSRef<CSourceProject>& Project,
    const CUTF8StringView& PackageName,
    const CUTF8StringView& DirPath,
    bool bReadOnly,
    CSourcePackage::SSettings Settings
) {
    TSRef<CSourcePackage> VersePackage = TSRef<CSourceDataPackage>::New(PackageName, DirPath, Settings);

    CSourceProject::SPackage NewPackage = {
        ._Package = VersePackage,
        ._bReadonly = bReadOnly,
    };

    return Project->_Packages.Add_GetRef(NewPackage);
}

struct FFI_PackageSettings {
    char* _VersePath;
    EVerseScope _VerseScope;
    EPackageRole _Role;
    bool _ExplicitVerseVersion;
    uint32_t _VerseVersion;
    bool _bTreatModulesAsImplicit;
    char** _DependencyPackages;
    size_t _DependencyPackagesLen;
    char* _VniDestDir;
    bool _bAllowExperimental;
};

extern "C" const CSourceProject::SPackage* Lsp_RegisterPackage(
    LspProjectContainer* ProjectContainer,
    const char* PackageName,
    const char* DirPath,
    const bool bReadOnly,
    FFI_PackageSettings Settings
) {
    uLang::TArray<CUTF8String> DependencyPackages;
    for (size_t Index = 0; Index < Settings._DependencyPackagesLen; Index++) {
        DependencyPackages.Add(CUTF8String(Settings._DependencyPackages[Index]));
    }

    CSourcePackage::SSettings PackageSettings = {
        ._VersePath = CUTF8String(Settings._VersePath),
        ._VerseScope = Settings._VerseScope,
        ._Role = Settings._Role,
        ._bTreatModulesAsImplicit = Settings._bTreatModulesAsImplicit,
        ._DependencyPackages = DependencyPackages,
        ._bAllowExperimental = Settings._bAllowExperimental,
    };
    if (Settings._ExplicitVerseVersion) {
        PackageSettings._VerseVersion = uLang::TOptional(Settings._VerseVersion);
    }
    if (Settings._VniDestDir) {
        PackageSettings._VniDestDir = uLang::TOptional(CUTF8String(Settings._VniDestDir));
    }

    const auto& NewPackage = RegisterPackage(ProjectContainer->_Project,
            CUTF8String(PackageName), CUTF8String(DirPath), bReadOnly, PackageSettings);

    return &NewPackage;
}

extern "C" void Lsp_UnregisterPackage(
    const CSourceProject::SPackage* Package
) {
    // TODO
    // const CSourceProject::SPackage& PackageRef = *Package;
    // GProjectContainer->_Project->_Packages.Remove(PackageRef);
}

extern "C" void Lsp_UpsertSource(
    const CSourceProject::SPackage* Package,
    const char* Path,
    const char* ModulePathToRoot,
    const char* Contents
) {
    TSRef<CSourceModule> Module = Package->_Package->_RootModule;
    FilePathUtils::ForeachPartOfPath(CUTF8String(ModulePathToRoot), [&Module](const CUTF8StringView& ModuleName) {
        if (ModuleName.IsFilled() && CSourceFileProject::IsValidModuleName(ModuleName)) {
            if (ModuleName == ".." || ModuleName == ".") {
                return;
            }
            auto ExistingModule = Module->FindSubmodule(ModuleName);
            if (ExistingModule) {
                Module = *ExistingModule;
            } else {
                TSRef<CSourceModule> NewModule = TSRef<CSourceModule>::New(ModuleName);
                Module->_Submodules.Add(NewModule);
                Module = NewModule;
            }
        }
    });

    CUTF8String SnippetPath = CUTF8String(Path);
    const auto& NewSnippet = TSRef<CSourceDataSnippet>::New(*SnippetPath, CUTF8String(Contents));

    auto PrevSnippet = Module->_SourceSnippets.FindByPredicate([&SnippetPath](ISourceSnippet* Candidate) -> bool {
        return Candidate->GetPath() == SnippetPath;
    });
    if (PrevSnippet) {
        Module->_SourceSnippets.Remove(*PrevSnippet);
    }

    Module->AddSnippet(NewSnippet);
}

