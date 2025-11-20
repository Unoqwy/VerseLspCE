#include "VerseLspCE.hpp"

#include "uLang/Common/Text/FilePathUtils.h"
#include "uLang/Common/Text/Symbol.h"
#include "uLang/Syntax/VstNode.h"
#include "uLang/Parser/ReservedSymbols.h"

using namespace Verse;
using namespace Verse::LspCE;

namespace Verse::LspCE
{

#define SEMANTIC_RESERVED_SYMBOLS(v) \
    v(_array, EReservedSymbol::Array) \
    v(_class, EReservedSymbol::Class) \

class CSemanticTokensVisitor final : public SAstVisitor {
public:
    CSemanticTokensVisitor(RsSemanticTokensAccumulator* TokenAccumulator, CSymbolTable& Symbols)
        : _TokenAccumulator(TokenAccumulator)
        , _ReservedSymbols(Symbols)
        {}

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
        case EAstNodeType::Identifier_BuiltInMacro:
            OutTokenKind = RsSemanticTokenKind::MACRO;
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
        case EAstNodeType::MacroCall:
            VisitMacroCall(static_cast<CExprMacroCall&>(AstNode));
            return;
        default:
            goto visit_all;
        }

        VstNode = AstNode.GetMappedVstNode();
        if (VstNode) {
            EmitToken(VstNode, OutTokenKind);
        }

    visit_all:
        VisitAll(AstNode);
    }

    void VisitAll(const CAstNode& AstNode) {
        AstNode.VisitImmediates(*this);
        AstNode.VisitChildren(*this);
    }

private:
    void VisitMacroCall(const CExprMacroCall& MacroCall) {
        const auto& MacroIdentifier = static_cast<CExprIdentifierBuiltInMacro&>(*MacroCall.Name());
        const CSymbol MacroSymbol = MacroIdentifier._Symbol;

        fprintf(stderr, "Macro symbol : %s\n", MacroSymbol.AsCString());
    }

private:
    RsSemanticTokensAccumulator* _TokenAccumulator;

    void EmitToken(const Vst::Node* OriginNode, RsSemanticTokenKind TokenKind) {
        RsSemanticTokenEntry TokenEntry = {
            ._TokenKind = TokenKind,
            ._Span = TextRangeToSpan(OriginNode->Whence()),
        };
        RS_AddSemanticToken(_TokenAccumulator, TokenEntry);
    }

    void EmitToken(const CAstNode& OriginAstNode, RsSemanticTokenKind TokenKind) {
        const Vst::Node* VstNode = OriginAstNode.GetMappedVstNode();
        if (VstNode) {
            EmitToken(VstNode, TokenKind);
        }
    }

    struct SReservedSymbols {
        SReservedSymbols(CSymbolTable& Symbols) {
        #define VISIT_SYMBOL(FieldName, Enum) FieldName = Symbols.AddChecked(GetReservedSymbol(Enum));
            SEMANTIC_RESERVED_SYMBOLS(VISIT_SYMBOL)
        #undef VISIT_SYMBOL
        }

    #define VISIT_SYMBOL(FieldName, Enum) CSymbol FieldName;
        SEMANTIC_RESERVED_SYMBOLS(VISIT_SYMBOL)
    #undef VISIT_SYMBOL
    } _ReservedSymbols;
};

class CVstSemanticTokensVisitor final {
public:
    CVstSemanticTokensVisitor(RsSemanticTokensAccumulator* TokenAccumulator, bool bAstFallback)
        : _TokenAccumulator(TokenAccumulator)
        , _bAstFallback(bAstFallback)
        {}

    void Visit(const Vst::Node& Node) {
        RsSemanticTokenKind OutTokenKind;

        switch (Node.GetElementType()) {
        case Vst::NodeType::Comment:
            OutTokenKind = RsSemanticTokenKind::COMMENT;
            break;
        case Vst::NodeType::Macro: {
            const Vst::Macro& MacroNode = Node.As<Vst::Macro>();

            if (const Vst::Identifier* MacroIdentifier = MacroNode.GetName()->AsNullable<Vst::Identifier>()) {
                const CUTF8String& MacroName = MacroIdentifier->GetSourceText();

                if (
                   MacroName == "using"

                || MacroName == "module"
                || MacroName == "class"
                || MacroName == "struct"
                || MacroName == "enum"
                || MacroName == "interface"

                || MacroName == "for"
                || MacroName == "loop"
                || MacroName == "case"

                || MacroName == "external"
                || MacroName == "profile"

                || MacroName == "branch"
                || MacroName == "spawn"
                || MacroName == "sync"
                || MacroName == "race"
                || MacroName == "rush"
                ) {
                    OutTokenKind = RsSemanticTokenKind::KEYWORD;
                } else if (_bAstFallback) {
                    OutTokenKind = RsSemanticTokenKind::MACRO;
                } else {
                    goto continue_visit;
                }

                EmitToken(*MacroIdentifier, OutTokenKind);
            }
            goto continue_visit;
        }
        default:
            goto continue_visit;
        }

        EmitToken(Node, OutTokenKind);

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

    bool _bAstFallback;

    void EmitToken(const Vst::Node& OriginNode, RsSemanticTokenKind TokenKind) {
        RsSemanticTokenEntry TokenEntry = {
            ._TokenKind = TokenKind,
            ._Span = TextRangeToSpan(OriginNode.Whence()),
        };
        RS_AddSemanticToken(_TokenAccumulator, TokenEntry);
    }
};

} // namespace Verse::LspCE

extern "C" void Lsp_SemanticTokens(
    LspProjectContainer* ProjectContainer,
    const CSourceProject::SPackage* Package,
    const char* Path,
    RsSemanticTokensAccumulator* TokenAccumulator
) {
    const Vst::Project& ProjectVst = *ProjectContainer->_BuildManager.GetProjectVst();

    CUTF8String SnippetPath = uLang::FilePathUtils::NormalizePath(CUTF8String(Path));
    const Vst::Snippet* SnippetVst = ProjectVst.FindSnippetByFilePath(SnippetPath);
    if (!SnippetVst) {
        fprintf(stderr, "\nCould not find snipppet\n\n");
        return;
    }

    const CAstNode* AstNode = SnippetVst->GetMappedAstNode();
    if (AstNode) {
        CVstSemanticTokensVisitor VstVisitor(TokenAccumulator, false);
        VstVisitor.Visit(*SnippetVst);

        CSemanticTokensVisitor AstVisitor(TokenAccumulator, *ProjectContainer->_Symbols);
        AstVisitor.VisitAll(*AstNode);
    } else {
        // TODO: Remove the fallback because Ast always seem to parse if Vst does
        //       Not sure what to do about the "down" time of syntax pass. May just be a flaw of using the compiler..
        CVstSemanticTokensVisitor VstVisitor(TokenAccumulator, true);
        VstVisitor.Visit(*SnippetVst);
    }
}
