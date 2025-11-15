using UnrealBuildTool;
using System.IO;


public class VerseLspCE : ModuleRules
{
	public VerseLspCE(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

		PrivateDependencyModuleNames.AddRange(
			new string[] {
				"Core",
				"uLangUE",
				"VerseCompiler",
			}
		);

		PublicAdditionalLibraries.Add(GetRustLibraryOutPath(Target));

		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			PublicSystemLibraries.AddRange(new string[] { "Userenv.lib", "ntdll.lib" });
		}
	}

	public string GetRustLibraryOutPath(ReadOnlyTargetRules Target)
	{
		string RustLibDir = Path.Combine(ModuleDirectory, "..", "verse_lsp_rs");
		RustLibDir = Path.GetFullPath(RustLibDir);
		if (!Directory.Exists(RustLibDir))
		{
			throw new BuildException($"Rust project directory not found at {RustLibDir}");
		}

		string TargetDir = Path.Combine(RustLibDir, "..", "target");
		TargetDir = Path.GetFullPath(TargetDir);

		if (Target.Configuration == UnrealTargetConfiguration.Shipping)
		{
			TargetDir = Path.Combine(TargetDir, "release");
		}
		else
		{
			TargetDir = Path.Combine(TargetDir, "debug");
		}

		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			return Path.Combine(TargetDir, "verse_lsp_rs.lib");
		}
		else
		{
			return Path.Combine(TargetDir, "libverse_lsp_rs.a");
		}
	}
}
