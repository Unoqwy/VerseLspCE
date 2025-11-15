#include <cstdint>


extern "C" {
    int RS_RunServer();

    struct RsDiagnosticAccumulator;

    struct RsDiagnostic {
        const char* _Path;
        const char* _Message;
        int32_t _Severity;
        uint32_t _BeginRow;
        uint32_t _BeginColumn;
        uint32_t _EndRow;
        uint32_t _EndColumn;
    };

    void RS_AddDiagnostic(RsDiagnosticAccumulator* DiagnosticAccumulator, RsDiagnostic Diagnostic);
}
