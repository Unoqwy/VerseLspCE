#include <cstdint>


extern "C" {
    int RS_RunServer();

    struct RsSourceSpan {
        uint32_t _BeginRow;
        uint32_t _BeginColumn;
        uint32_t _EndRow;
        uint32_t _EndColumn;
    };

    // Diagnostics {{{
    struct RsDiagnosticAccumulator;

    struct RsDiagnostic {
        const char* _Path;
        const char* _Message;
        uint16_t _ReferenceCode;
        int32_t _Severity;
        RsSourceSpan _Span;
    };

    void RS_AddDiagnostic(RsDiagnosticAccumulator* DiagnosticAccumulator, RsDiagnostic Diagnostic);
    // }}}

    // Semantic Tokens {{{
    struct RsSemanticTokensAccumulator;

    enum RsSemanticTokenKind : uint32_t {
        NAMESPACE,
        TYPE,
        ENUM,
        ENUMMEMBER,
        STRUCT,
        CLASS,
        INTERFACE,
        PARAMETER,
        TYPEPARAMETER,
        PROPERTY,
        VARIABLE,
        FUNCTION,
        METHOD,
        MACRO,
        KEYWORD,
        COMMENT,
        STRING,
        NUMBER,
        OPERATOR,
        ATTRIBUTE,
        SPECIFIER,
    };

    struct RsSemanticTokenEntry {
        RsSemanticTokenKind _TokenKind;
        RsSourceSpan _Span;
    };

    void RS_AddSemanticToken(RsSemanticTokensAccumulator* TokenAccumulator, RsSemanticTokenEntry TokenEntry);
    // }}}
}
