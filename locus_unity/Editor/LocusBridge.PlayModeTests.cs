using UnityEditor;
using UnityEngine;

using System;
using System.IO;
using System.Reflection;

namespace Locus
{
    public static partial class LocusBridge
    {
        private const string SessionKey_PlayModeTestGuid = "Locus_PlayModeTestGuid";
        private const string SessionKey_PlayModeTestState = "Locus_PlayModeTestState";
        private const string SessionKey_PlayModeTestResultFile = "Locus_PlayModeTestResultFile";
        private const string SessionKey_PlayModeTestSummary = "Locus_PlayModeTestSummary";
        private const string SessionKey_PlayModeTestError = "Locus_PlayModeTestError";
        private const string SessionKey_PlayModeTestUpdatedAt = "Locus_PlayModeTestUpdatedAt";

        private static object _playModeCallbacksProxy;
        private static Type _testRunnerApiType;
        private static Type _executionSettingsType;
        private static Type _filterType;
        private static Type _testModeType;
        private static Type _iCallbacksType;
        private static MethodInfo _registerTestCallbackMethod;
        private static MethodInfo _executeTestRunMethod;
        private static MethodInfo _isRunActiveMethod;
        private static MethodInfo _cancelTestRunMethod;
        private static MethodInfo _saveResultToFileMethod;
        private static bool _playModeApiResolved;

        private static bool EnsurePlayModeTestsApiResolved(out string error)
        {
            error = null;
            if (_playModeApiResolved && _testRunnerApiType != null)
                return true;

            try
            {
                _testRunnerApiType = ResolveType(
                    "UnityEditor.TestTools.TestRunner.Api.TestRunnerApi",
                    "UnityEditor.TestRunner",
                    "solution");
                _executionSettingsType = ResolveType(
                    "UnityEditor.TestTools.TestRunner.Api.ExecutionSettings",
                    "UnityEditor.TestRunner",
                    "solution");
                _filterType = ResolveType(
                    "UnityEditor.TestTools.TestRunner.Api.Filter",
                    "UnityEditor.TestRunner",
                    "solution");
                _testModeType = ResolveType(
                    "UnityEditor.TestTools.TestRunner.Api.TestMode",
                    "UnityEditor.TestRunner",
                    "solution");
                _iCallbacksType = ResolveType(
                    "UnityEditor.TestTools.TestRunner.Api.ICallbacks",
                    "UnityEditor.TestRunner",
                    "solution");

                if (_testRunnerApiType == null || _executionSettingsType == null || _filterType == null ||
                    _testModeType == null || _iCallbacksType == null)
                {
                    error = "Unity Test Framework API is unavailable. Please install package 'com.unity.test-framework' via Package Manager, then reopen Unity.";
                    return false;
                }

                _executeTestRunMethod = _testRunnerApiType.GetMethod(
                    "ExecuteTestRun",
                    BindingFlags.Public | BindingFlags.Static,
                    null,
                    new[] { _executionSettingsType },
                    null);
                _isRunActiveMethod = _testRunnerApiType.GetMethod(
                    "IsRunActive",
                    BindingFlags.Public | BindingFlags.Static,
                    null,
                    new[] { typeof(string) },
                    null);
                _cancelTestRunMethod = _testRunnerApiType.GetMethod(
                    "CancelTestRun",
                    BindingFlags.Public | BindingFlags.Static,
                    null,
                    new[] { typeof(string) },
                    null);
                _saveResultToFileMethod = ResolveSaveResultToFileMethod(_testRunnerApiType);
                _registerTestCallbackMethod = ResolveRegisterCallbackMethod(_testRunnerApiType, _iCallbacksType);

                if (_executeTestRunMethod == null || _isRunActiveMethod == null || _registerTestCallbackMethod == null)
                {
                    error = "Unity Test Framework API is present but incompatible with this Unity/TestFramework version. Update 'com.unity.test-framework' and reopen Unity.";
                    return false;
                }

                _playModeApiResolved = true;
                return true;
            }
            catch (Exception ex)
            {
                error = "Failed to resolve Unity Test Framework API: " + ex.Message;
                return false;
            }
        }

        private static Type ResolveType(string fullName, params string[] assemblyHints)
        {
            foreach (var asm in AppDomain.CurrentDomain.GetAssemblies())
            {
                try
                {
                    var t = asm.GetType(fullName, false);
                    if (t != null)
                        return t;
                }
                catch
                {
                }
            }

            if (assemblyHints != null)
            {
                for (int i = 0; i < assemblyHints.Length; i++)
                {
                    string hint = assemblyHints[i];
                    if (string.IsNullOrEmpty(hint))
                        continue;
                    try
                    {
                        var resolved = Type.GetType(fullName + ", " + hint, false);
                        if (resolved != null)
                            return resolved;
                    }
                    catch
                    {
                    }
                }
            }

            return null;
        }

        private static MethodInfo ResolveRegisterCallbackMethod(Type apiType, Type callbacksType)
        {
            foreach (var method in apiType.GetMethods(BindingFlags.Public | BindingFlags.Static))
            {
                if (!string.Equals(method.Name, "RegisterTestCallback", StringComparison.Ordinal))
                    continue;
                if (!method.IsGenericMethodDefinition)
                    continue;
                var parameters = method.GetParameters();
                if (parameters.Length < 1)
                    continue;
                var generic = method.MakeGenericMethod(callbacksType);
                return generic;
            }

            return null;
        }

        private static MethodInfo ResolveSaveResultToFileMethod(Type apiType)
        {
            foreach (var method in apiType.GetMethods(BindingFlags.Public | BindingFlags.Static))
            {
                if (!string.Equals(method.Name, "SaveResultToFile", StringComparison.Ordinal))
                    continue;
                var parameters = method.GetParameters();
                if (parameters.Length == 2 && parameters[1].ParameterType == typeof(string))
                    return method;
            }

            return null;
        }

        private static bool EnsurePlayModeTestCallbacksRegistered(out string error)
        {
            error = null;
            if (!EnsurePlayModeTestsApiResolved(out error))
                return false;

            if (_playModeCallbacksProxy != null)
                return true;

            try
            {
                PlayModeTestDispatchProxy.Handler = HandlePlayModeTestCallback;
                _playModeCallbacksProxy = CreateDispatchProxy(_iCallbacksType, typeof(PlayModeTestDispatchProxy));
                _registerTestCallbackMethod.Invoke(null, new object[] { _playModeCallbacksProxy, 0 });
                return true;
            }
            catch (Exception ex)
            {
                _playModeCallbacksProxy = null;
                error = "Failed to register Unity test callbacks: " + ex.Message;
                return false;
            }
        }

        private static object CreateDispatchProxy(Type interfaceType, Type proxyType)
        {
            MethodInfo createMethod = typeof(DispatchProxy).GetMethod("Create", BindingFlags.Public | BindingFlags.Static);
            MethodInfo genericCreateMethod = createMethod.MakeGenericMethod(interfaceType, proxyType);
            return genericCreateMethod.Invoke(null, null);
        }

        private static void HandlePlayModeTestCallback(string methodName, object[] args)
        {
            try
            {
                if (string.Equals(methodName, "RunStarted", StringComparison.Ordinal))
                {
                    string guid = SessionState.GetString(SessionKey_PlayModeTestGuid, "");
                    SetPlayModeTestState(guid, "running", null, null, null);
                    return;
                }

                if (string.Equals(methodName, "RunFinished", StringComparison.Ordinal))
                {
                    object resultAdaptor = (args != null && args.Length > 0) ? args[0] : null;
                    string guid = SessionState.GetString(SessionKey_PlayModeTestGuid, "");
                    string summary = BuildPlayModeTestSummary(resultAdaptor);
                    string resultFile = SavePlayModeResultFile(resultAdaptor, guid);
                    SetPlayModeTestState(guid, "finished", summary, resultFile, null);
                    return;
                }
            }
            catch (Exception ex)
            {
                string guid = SessionState.GetString(SessionKey_PlayModeTestGuid, "");
                SetPlayModeTestState(guid, "failed", null, null, ex.Message);
            }
        }

        private static string BuildPlayModeTestSummary(object resultAdaptor)
        {
            if (resultAdaptor == null)
                return "no_result";

            try
            {
                Type t = resultAdaptor.GetType();
                string resultState = SafeGetPropertyString(t, resultAdaptor, "ResultState");
                long passCount = SafeGetPropertyInt64(t, resultAdaptor, "PassCount");
                long failCount = SafeGetPropertyInt64(t, resultAdaptor, "FailCount");
                long skipCount = SafeGetPropertyInt64(t, resultAdaptor, "SkipCount");
                return string.Format(
                    "state={0}; passed={1}; failed={2}; skipped={3}",
                    string.IsNullOrEmpty(resultState) ? "unknown" : resultState,
                    passCount,
                    failCount,
                    skipCount);
            }
            catch (Exception ex)
            {
                return "summary_error:" + ex.Message;
            }
        }

        private static string SafeGetPropertyString(Type t, object instance, string name)
        {
            var p = t.GetProperty(name, BindingFlags.Public | BindingFlags.Instance);
            if (p == null)
                return "";
            object value = p.GetValue(instance, null);
            return value != null ? value.ToString() : "";
        }

        private static long SafeGetPropertyInt64(Type t, object instance, string name)
        {
            var p = t.GetProperty(name, BindingFlags.Public | BindingFlags.Instance);
            if (p == null)
                return 0;
            object value = p.GetValue(instance, null);
            if (value == null)
                return 0;
            try
            {
                return Convert.ToInt64(value);
            }
            catch
            {
                return 0;
            }
        }

        private static string SavePlayModeResultFile(object resultAdaptor, string guid)
        {
            if (_saveResultToFileMethod == null || resultAdaptor == null)
                return "";

            try
            {
                string projectPath = Directory.GetParent(Application.dataPath).FullName;
                string dir = Path.Combine(projectPath, "Library", "Locus", "TestResults");
                Directory.CreateDirectory(dir);
                string normalizedGuid = string.IsNullOrEmpty(guid) ? "unknown" : guid;
                string filePath = Path.Combine(dir, "playmode-" + normalizedGuid + "-" + DateTime.UtcNow.ToString("yyyyMMddHHmmss") + ".xml");
                _saveResultToFileMethod.Invoke(null, new object[] { resultAdaptor, filePath });
                return filePath.Replace('\\', '/');
            }
            catch
            {
                return "";
            }
        }

        private static void SetPlayModeTestState(string guid, string state, string summary, string resultFile, string error)
        {
            if (!string.IsNullOrEmpty(guid))
                SessionState.SetString(SessionKey_PlayModeTestGuid, guid);
            SessionState.SetString(SessionKey_PlayModeTestState, state ?? "");
            SessionState.SetString(SessionKey_PlayModeTestSummary, summary ?? "");
            SessionState.SetString(SessionKey_PlayModeTestResultFile, resultFile ?? "");
            SessionState.SetString(SessionKey_PlayModeTestError, error ?? "");
            SessionState.SetString(SessionKey_PlayModeTestUpdatedAt, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds().ToString());
        }

        private static long GetPlayModeUpdatedAtMs()
        {
            string raw = SessionState.GetString(SessionKey_PlayModeTestUpdatedAt, "");
            long value;
            if (long.TryParse(raw, out value))
                return value;
            return 0;
        }

        private static PipeEnvelope HandlePlayModeTestsStart(string reqId, string payload)
        {
            string error;
            if (!EnsurePlayModeTestCallbacksRegistered(out error))
                return ErrorResponse(reqId, error);

            PlayModeTestsRequest request = null;
            if (!string.IsNullOrEmpty(payload))
            {
                try { request = JsonUtility.FromJson<PlayModeTestsRequest>(payload); } catch { }
            }
            if (request == null)
                request = new PlayModeTestsRequest();

            try
            {
                object filter = Activator.CreateInstance(_filterType);
                PropertyInfo testModeProperty = _filterType.GetProperty("testMode", BindingFlags.Public | BindingFlags.Instance);
                if (testModeProperty != null)
                {
                    object playModeEnum = Enum.Parse(_testModeType, "PlayMode");
                    testModeProperty.SetValue(filter, playModeEnum, null);
                }

                if (request.testNames != null && request.testNames.Length > 0)
                {
                    PropertyInfo namesProp = _filterType.GetProperty("testNames", BindingFlags.Public | BindingFlags.Instance);
                    if (namesProp != null)
                        namesProp.SetValue(filter, request.testNames, null);
                }

                if (request.assemblyNames != null && request.assemblyNames.Length > 0)
                {
                    PropertyInfo asmProp = _filterType.GetProperty("assemblyNames", BindingFlags.Public | BindingFlags.Instance);
                    if (asmProp != null)
                        asmProp.SetValue(filter, request.assemblyNames, null);
                }

                object executionSettings = Activator.CreateInstance(_executionSettingsType, new object[] { filter });
                string guid = _executeTestRunMethod.Invoke(null, new object[] { executionSettings }) as string;
                if (string.IsNullOrEmpty(guid))
                    return ErrorResponse(reqId, "Unity TestRunner returned an empty run guid.");

                SetPlayModeTestState(guid, "running", null, null, null);
                var responsePayload = new PlayModeTestsStatusPayload
                {
                    guid = guid,
                    state = "running",
                    active = true,
                    resultFile = "",
                    summary = "",
                    error = "",
                    updatedAtMs = GetPlayModeUpdatedAtMs(),
                };
                return OkResponse(reqId, JsonUtility.ToJson(responsePayload));
            }
            catch (Exception ex)
            {
                SetPlayModeTestState("", "failed", null, null, ex.Message);
                return ErrorResponse(reqId, "Failed to start PlayMode tests: " + ex.Message);
            }
        }

        private static PipeEnvelope HandlePlayModeTestsStatus(string reqId)
        {
            string guid = SessionState.GetString(SessionKey_PlayModeTestGuid, "");
            string state = SessionState.GetString(SessionKey_PlayModeTestState, "");
            string resultFile = SessionState.GetString(SessionKey_PlayModeTestResultFile, "");
            string summary = SessionState.GetString(SessionKey_PlayModeTestSummary, "");
            string error = SessionState.GetString(SessionKey_PlayModeTestError, "");
            bool active = false;

            if (!string.IsNullOrEmpty(guid))
            {
                try
                {
                    string apiError;
                    if (EnsurePlayModeTestsApiResolved(out apiError) && _isRunActiveMethod != null)
                    {
                        object value = _isRunActiveMethod.Invoke(null, new object[] { guid });
                        active = value is bool b && b;
                    }
                }
                catch
                {
                    active = false;
                }
            }

            if (active && string.IsNullOrEmpty(state))
                state = "running";
            if (!active && string.Equals(state, "running", StringComparison.Ordinal))
                state = "finished";

            var payload = new PlayModeTestsStatusPayload
            {
                guid = guid ?? "",
                state = string.IsNullOrEmpty(state) ? "idle" : state,
                active = active,
                resultFile = resultFile ?? "",
                summary = summary ?? "",
                error = error ?? "",
                updatedAtMs = GetPlayModeUpdatedAtMs(),
            };
            return OkResponse(reqId, JsonUtility.ToJson(payload));
        }

        private static PipeEnvelope HandlePlayModeTestsCancel(string reqId)
        {
            string guid = SessionState.GetString(SessionKey_PlayModeTestGuid, "");
            if (string.IsNullOrEmpty(guid))
                return OkResponse(reqId, "{\"state\":\"idle\",\"active\":false}");

            try
            {
                string apiError;
                bool cancelled = false;
                if (EnsurePlayModeTestsApiResolved(out apiError) && _cancelTestRunMethod != null)
                {
                    object value = _cancelTestRunMethod.Invoke(null, new object[] { guid });
                    cancelled = value is bool b && b;
                }
                SetPlayModeTestState(guid, "cancelled", null, null, cancelled ? null : "Cancel was not acknowledged by Unity TestRunner.");
                return HandlePlayModeTestsStatus(reqId);
            }
            catch (Exception ex)
            {
                return ErrorResponse(reqId, "Failed to cancel PlayMode tests: " + ex.Message);
            }
        }

        private sealed class PlayModeTestDispatchProxy : DispatchProxy
        {
            public static Action<string, object[]> Handler;

            protected override object Invoke(MethodInfo targetMethod, object[] args)
            {
                try
                {
                    if (Handler != null && targetMethod != null)
                        Handler(targetMethod.Name, args);
                }
                catch (Exception ex)
                {
                    Debug.LogError("[Locus] PlayMode test callback proxy failed: " + ex);
                }

                return null;
            }
        }
    }
}
