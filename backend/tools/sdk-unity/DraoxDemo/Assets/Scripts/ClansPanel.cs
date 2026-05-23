using System;
using Cysharp.Threading.Tasks;
using Draox.Client.Plugins;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

namespace Draox.Demo
{
    /// <summary>
    /// Panel: exercise the ClansPlugin — list, create, join, leave.
    /// </summary>
    public class ClansPanel : MonoBehaviour
    {
        [Header("List")]
        [SerializeField] private Button          listButton;
        [SerializeField] private TextMeshProUGUI listResultText;

        [Header("Create")]
        [SerializeField] private TMP_InputField clanNameInput;
        [SerializeField] private TMP_InputField clanTagInput;
        [SerializeField] private TMP_InputField clanDescInput;
        [SerializeField] private Button         createButton;

        [Header("Join / Leave")]
        [SerializeField] private TMP_InputField clanIdInput;
        [SerializeField] private Button         joinButton;
        [SerializeField] private Button         leaveButton;

        [Header("Kick / Promote")]
        [SerializeField] private TMP_InputField  targetUserInput;
        [SerializeField] private TMP_InputField  promoteRoleInput;
        [SerializeField] private Button          kickButton;
        [SerializeField] private Button          promoteButton;

        private ClansPlugin _clans;

        private void Start()
        {
            _clans = DemoManager.Instance.Clans;

            _clans.OnJoined        += e => Log($"Joined clan {e.ClanName} ({e.ClanId})", LogLevel.Event);
            _clans.OnLeft          += e => Log($"Left clan {e.ClanId}  reason={e.Reason}", LogLevel.Event);
            _clans.OnMemberChanged += e => Log($"Member {e.Username} {e.Action} in {e.ClanId}", LogLevel.Event);

            listButton?.onClick.AddListener(() => ListAsync().Forget());
            createButton?.onClick.AddListener(() => CreateAsync().Forget());
            joinButton?.onClick.AddListener(() => JoinAsync().Forget());
            leaveButton?.onClick.AddListener(() => LeaveAsync().Forget());
            kickButton?.onClick.AddListener(() => KickAsync().Forget());
            promoteButton?.onClick.AddListener(() => PromoteAsync().Forget());

            if (clanNameInput    != null) clanNameInput.text    = "MyTestClan";
            if (clanTagInput     != null) clanTagInput.text     = "MTC";
            if (promoteRoleInput != null) promoteRoleInput.text = "officer";
        }

        private async UniTaskVoid ListAsync()
        {
            Log("Listing clans …");
            try
            {
                var res = await _clans.ListClansAsync();
                if (listResultText != null)
                {
                    var sb = new System.Text.StringBuilder();
                    sb.AppendLine($"Total: {res.Total}");
                    if (res.Clans != null)
                        foreach (var c in res.Clans)
                            sb.AppendLine($"  [{c.Tag}] {c.Name}  ({c.MemberCount} members)  id={c.Id}");
                    listResultText.text = sb.ToString();
                }
                Log($"Got {res.Total} clan(s)", LogLevel.Success);
            }
            catch (Exception ex) { Log($"List error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid CreateAsync()
        {
            var name = clanNameInput?.text?.Trim();
            var tag  = clanTagInput?.text?.Trim();
            var desc = clanDescInput?.text?.Trim();
            if (string.IsNullOrEmpty(name)) return;

            Log($"Creating clan \"{name}\" …");
            try
            {
                var res = await _clans.CreateClanAsync(name, tag, desc);
                Log($"Clan created: {res.Info?.Name}  id={res.Info?.Id}", LogLevel.Success);
                if (clanIdInput != null) clanIdInput.text = res.Info?.Id ?? string.Empty;
            }
            catch (Exception ex) { Log($"Create error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid JoinAsync()
        {
            var id = clanIdInput?.text?.Trim();
            if (string.IsNullOrEmpty(id)) return;

            Log($"Joining clan {id} …");
            try
            {
                await _clans.JoinClanAsync(id);
                Log("Join request sent.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Join error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid LeaveAsync()
        {
            Log("Leaving current clan …");
            try
            {
                await _clans.LeaveClanAsync();
                Log("Left clan.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Leave error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid KickAsync()
        {
            var userId = targetUserInput?.text?.Trim();
            if (string.IsNullOrEmpty(userId)) return;

            Log($"Kicking {userId} …");
            try
            {
                await _clans.KickMemberAsync(userId);
                Log($"Kicked {userId}.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Kick error: {ex.Message}", LogLevel.Error); }
        }

        private async UniTaskVoid PromoteAsync()
        {
            var userId = targetUserInput?.text?.Trim();
            var role   = promoteRoleInput?.text?.Trim() ?? "officer";
            if (string.IsNullOrEmpty(userId)) return;

            Log($"Promoting {userId} → {role} …");
            try
            {
                await _clans.PromoteAsync(userId, role);
                Log($"Promoted {userId} to {role}.", LogLevel.Success);
            }
            catch (Exception ex) { Log($"Promote error: {ex.Message}", LogLevel.Error); }
        }

        private void Log(string msg, LogLevel level = LogLevel.Info) =>
            DemoManager.Instance.Log(msg, level);
    }
}
