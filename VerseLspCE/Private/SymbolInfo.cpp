#include "VerseLspCE.hpp"
#include "uLang/Semantics/Expression.h"

using namespace Verse::LspCE;


class CFindSnippetByPathVisitor final : public SAstVisitor {
public:
    CExprSnippet* _SnippetNode = nullptr;

    CFindSnippetByPathVisitor(const CUTF8StringView& Path)
        : Path(Path) {}

    virtual void Visit(const char* /*FieldName*/, CAstNode& AstNode) override {
        fprintf(stderr, "NODE %s \n", GetAstNodeTypeInfo(AstNode.GetNodeType())._EnumeratorName);
        if (AstNode.GetNodeType() == EAstNodeType::Context_Snippet) {
            auto& SnippetNode = static_cast<CExprSnippet&>(AstNode);
            fprintf(stderr, "PATH %s \n", SnippetNode._Path.AsCString());
            if (SnippetNode._Path == Path) {
                _SnippetNode = &SnippetNode;
            }
        }
        if (AstNode.GetNodeType() == EAstNodeType::Definition_Module) {
            auto& ModuleNode = static_cast<CExprModuleDefinition&>(AstNode);
            fprintf(stderr, "Module : %s\n", ModuleNode._Name.AsCString());
        }
    }

    virtual void VisitElement(CAstNode& AstNode) override {
        Visit(AstNode);
    }

    void Visit(const CAstNode& AstNode) {
        if (_SnippetNode) {
            return;
        }
        AstNode.VisitImmediates(*this);
        AstNode.VisitChildren(*this);
    }

private:
    const CUTF8StringView& Path;
};

extern "C" void Lsp_SymbolInfo(
    LspProjectContainer* ProjectContainer,
    const CSourceProject::SPackage* Package,
    const char* Path
) {
    if (!ProjectContainer->_LastAstProject) {
        return;
    }

    const CAstProject& AstProject = *ProjectContainer->_LastAstProject;

    const CUTF8String& ProjectName = Package->_Package->GetName();
    const CAstPackage* AstPackage = AstProject.FindPackageByName(ProjectName);
    if (!AstPackage) {
        return;
    }

    CUTF8String SnippetPath = CUTF8String(Path);
    CFindSnippetByPathVisitor Visitor(SnippetPath);
    Visitor.Visit(*AstPackage);

    if (!Visitor._SnippetNode) {
        fprintf(stderr, "\n\nCould not find snipppet\n");
        return;
    }

    const CExprSnippet& SnippetNode = *Visitor._SnippetNode;
    // TODO:
}
