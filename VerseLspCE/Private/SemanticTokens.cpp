#include "VerseLspCE.hpp"

#include "uLang/Syntax/VstNode.h"

using namespace Verse;
using namespace Verse::LspCE;


class CSemanticTokensVisitor final : public SAstVisitor {
public:
    CSemanticTokensVisitor(RsSemanticTokensAccumulator* TokenAccumulator)
        : _TokenAccumulator(TokenAccumulator) {}

    virtual void Visit(const char* /*FieldName*/, CAstNode& AstNode) override {
        VisitElement(AstNode);
    }

    virtual void VisitElement(CAstNode& AstNode) override {
        // fprintf(stderr, "NODE %s \n", GetAstNodeTypeInfo(AstNode.GetNodeType())._EnumeratorName);

        const Vst::Node* VstNode;
        RsSemanticTokenKind OutTokenKind;

        switch (AstNode.GetNodeType()) {
        case EAstNodeType::Literal_Path:
        case EAstNodeType::Identifier_Module:
        case EAstNodeType::Identifier_ModuleAlias:
            OutTokenKind = RsSemanticTokenKind::NAMESPACE;
            break;
        case EAstNodeType::Identifier_Enum:
            OutTokenKind = RsSemanticTokenKind::ENUM;
            break;
        case EAstNodeType::Identifier_Class:
            OutTokenKind = RsSemanticTokenKind::CLASS;
            break;
        case EAstNodeType::Identifier_Interface:
            OutTokenKind = RsSemanticTokenKind::INTERFACE;
            break;
        case EAstNodeType::Identifier_Function:
        case EAstNodeType::Identifier_OverloadedFunction:
            OutTokenKind = RsSemanticTokenKind::FUNCTION;
            break;
        case EAstNodeType::Literal_String:
        case EAstNodeType::Literal_Char:
            OutTokenKind = RsSemanticTokenKind::STRING;
            break;
        case EAstNodeType::Literal_Number:
            OutTokenKind = RsSemanticTokenKind::NUMBER;
            break;
        default:
            goto continue_visit;
        }

        VstNode = AstNode.GetMappedVstNode();
        if (VstNode) {
            RsSemanticTokenEntry TokenEntry = {
                ._TokenKind = OutTokenKind,
                ._Span = TextRangeToSpan(VstNode->Whence()),
            };
            RS_AddSemanticToken(_TokenAccumulator, TokenEntry);
        }

    continue_visit:
        VisitAll(AstNode);
    }

    void VisitAll(const CAstNode& AstNode) {
        AstNode.VisitImmediates(*this);
        AstNode.VisitChildren(*this);
    }

private:
    RsSemanticTokensAccumulator* _TokenAccumulator;
};

class CVstSemanticTokensVisitor final {
public:
    CVstSemanticTokensVisitor(RsSemanticTokensAccumulator* TokenAccumulator)
        : _TokenAccumulator(TokenAccumulator) {}

    void Visit(const Vst::Node& Node) {
        RsSemanticTokenKind OutTokenKind;

        switch (Node.GetElementType()) {
        case Vst::NodeType::Comment:
            OutTokenKind = RsSemanticTokenKind::COMMENT;
            break;
        default:
            goto continue_visit;
        }

    {
        RsSemanticTokenEntry TokenEntry = {
            ._TokenKind = OutTokenKind,
            ._Span = TextRangeToSpan(Node.Whence()),
        };
        RS_AddSemanticToken(_TokenAccumulator, TokenEntry);
    }

    continue_visit:
        for (const auto& Child : Node.GetPrefixComments()) {
            Visit(*Child);
        }
        for (const auto& Child : Node.GetChildren()) {
            Visit(*Child);
        }
        for (const auto& Child : Node.GetPostfixComments()) {
            Visit(*Child);
        }
    }

private:
    RsSemanticTokensAccumulator* _TokenAccumulator;
};

extern "C" void Lsp_SemanticTokens(
    LspProjectContainer* ProjectContainer,
    const CSourceProject::SPackage* Package,
    const char* Path,
    RsSemanticTokensAccumulator* TokenAccumulator
) {
    const Vst::Project& ProjectVst = *ProjectContainer->_BuildManager.GetProjectVst();

    CUTF8String SnippetPath = CUTF8String(Path);
    const Vst::Snippet* SnippetVst = ProjectVst.FindSnippetByFilePath(SnippetPath);
    if (!SnippetVst) {
        fprintf(stderr, "\nCould not find snipppet\n\n");
        return;
    }

    CVstSemanticTokensVisitor VstVisitor(TokenAccumulator);
    VstVisitor.Visit(*SnippetVst);

    const CAstNode* AstNode = SnippetVst->GetMappedAstNode();
    if (AstNode) {
        CSemanticTokensVisitor AstVisitor(TokenAccumulator);
        AstVisitor.VisitAll(*AstNode);
    }
}
