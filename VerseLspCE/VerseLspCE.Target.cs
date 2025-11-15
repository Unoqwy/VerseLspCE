using UnrealBuildTool;
using System;
using System.IO;
using System.Runtime.CompilerServices;
using System.Diagnostics;


public class VerseLspCETarget : TargetRules
{
	public VerseLspCETarget(TargetInfo Target) : base(Target)
	{
		Type = TargetType.Program;
		LinkType = TargetLinkType.Monolithic;
		CppStandard = CppStandardVersion.Cpp20;
		MinCpuArchX64 = MinimumCpuArchitectureX64.AVX;

		LaunchModuleName = "VerseLspCE";
		bIsBuildingConsoleApplication = true;

		IncludeOrderVersion = EngineIncludeOrderVersion.Latest;

		bCompileAgainstEngine = false;
		bCompileAgainstCoreUObject = true;

		bUseUnityBuild = false;
		bCompileICU = false;
		bEnableTrace = false;

		bUseVerseBPVM = true;

		bFNameOutlineNumber = true;

		bCompileWithStatsWithoutEngine = true;
		GlobalDefinitions.Add("ENABLE_STATNAMEDEVENTS=1");
		GlobalDefinitions.Add("ENABLE_STATNAMEDEVENTS_UOBJECT=1");

		GlobalDefinitions.Add("MALLOC_LEAKDETECTION=1");
		GlobalDefinitions.Add("PLATFORM_USES_FIXED_GMalloc_CLASS=0");

		string ModuleDirectory = GetModuleDirectory();

		AddPreBuildSteps(ModuleDirectory, Target);
	}

	// Target.ProjectFile is null so we can't use that
	private string GetModuleDirectory([CallerFilePath] string ThisTargetFile = "")
	{
		var ModuleDirectoryInfo = Directory.GetParent(ThisTargetFile);
		if (ModuleDirectoryInfo == null)
		{
			throw new BuildException($"Couldn't retrieve module directory");
		}
		return ModuleDirectoryInfo.FullName;
	}

	private void AddPreBuildSteps(string ModuleDirectory, TargetInfo Target)
	{
		string RustLibDir = Path.Combine(ModuleDirectory, "..");
		RustLibDir = Path.GetFullPath(RustLibDir);

		string ConfigurationFlag = "";
		if (Target.Configuration == UnrealTargetConfiguration.Shipping)
		{
			ConfigurationFlag = " --release";
		}

		if (Target.Platform == UnrealTargetPlatform.Win64)
		{
			PreBuildSteps.Add($"cmd /c \"cd /d \"{RustLibDir}\" && cargo build -p verse_lsp_rs{ConfigurationFlag}\"");
		}
		else
		{
			PreBuildSteps.Add($"cd \"{RustLibDir}\" && cargo build -p verse_lsp_rs{ConfigurationFlag}");
		}
	}
}
