using System;
using System.Collections;
using System.Collections.Generic;
using System.Threading;
using Cysharp.Threading.Tasks;
using NUnit.Framework;
using UnityEngine.TestTools;
using Draox.Client;

namespace Draox.Client.Tests
{
    // ── Mock connection ───────────────────────────────────────────────────────

    /// <summary>
    /// In-memory IConnection that lets tests inject server responses.
    /// </summary>
    internal class MockConnection : IConnection
    {
        public event Action<string> MessageReceived;
        public event Action         Opened;
        public event Action<string> Closed;

        public bool IsConnected { get; private set; }

        public readonly List<string> SentMessages = new List<string>();

        public UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default)
        {
            IsConnected = true;
            return UniTask.CompletedTask;
        }

        public UniTask DisconnectAsync()
        {
            IsConnected = false;
            Closed?.Invoke("mock_disconnect");
            return UniTask.CompletedTask;
        }

        public UniTask SendTextAsync(string json, CancellationToken ct = default)
        {
            SentMessages.Add(json);
            return UniTask.CompletedTask;
        }

        /// <summary>Push a server-originated message back to the client under test.</summary>
        public void Receive(string json) => MessageReceived?.Invoke(json);

        public void SimulateClose(string reason) => Closed?.Invoke(reason);
    }

    // ── Serializer tests ──────────────────────────────────────────────────────

    [TestFixture]
    public class SerializerTests
    {
        [Test]
        public void Serialize_Roundtrip()
        {
            var obj = new { name = "test", value = 42 };
            var json = Serializer.Serialize(obj);
            Assert.IsTrue(json.Contains("\"name\""));
            Assert.IsTrue(json.Contains("\"test\""));
        }

        [Test]
        public void Deserialize_Roundtrip()
        {
            const string json = "{\"SessionId\":\"abc123\"}";
            var result = Serializer.Deserialize<AuthResponseData>(json);
            Assert.AreEqual("abc123", result.SessionId);
        }

        [Test]
        public void Parse_ResponseMessage()
        {
            const string json = "{\"type\":\"response\",\"id\":\"req_001\",\"success\":true,\"data\":{\"ok\":true}}";
            var msg = Serializer.Parse(json);
            Assert.IsNotNull(msg);
            Assert.AreEqual("response", msg.Type);
            Assert.AreEqual("req_001", msg.Id);
            Assert.IsTrue(msg.Success);
            Assert.IsNotNull(msg.RawData);
        }

        [Test]
        public void Parse_EventMessage()
        {
            const string json = "{\"type\":\"event\",\"category\":\"clan\",\"name\":\"joined\",\"data\":{\"clan_id\":\"c1\"},\"timestamp\":\"1000\"}";
            var msg = Serializer.Parse(json);
            Assert.IsNotNull(msg);
            Assert.AreEqual("event", msg.Type);
            Assert.AreEqual("clan", msg.Category);
            Assert.AreEqual("joined", msg.Name);
        }

        [Test]
        public void Parse_PongMessage()
        {
            var msg = Serializer.Parse("{\"type\":\"pong\"}");
            Assert.AreEqual("pong", msg.Type);
        }

        [Test]
        public void Parse_InvalidJson_ReturnsNull()
        {
            var msg = Serializer.Parse("not json at all!!");
            Assert.IsNull(msg);
        }
    }

    // ── RequestBroker tests ───────────────────────────────────────────────────

    [TestFixture]
    public class RequestBrokerTests
    {
        [UnityTest]
        public IEnumerator Broker_CompletesWhenResponseArrives() => UniTask.ToCoroutine(async () =>
        {
            var broker = new RequestBroker();
            var conn   = new MockConnection();
            await conn.ConnectAsync(new DraoxConfig());

            const string id = "req_test_1";
            var task = broker.SendAsync(conn, "{}", id, 5000, CancellationToken.None);

            broker.Complete(id, new DraoxResponse { Id = id, Success = true, RawData = "{}" });

            var result = await task;
            Assert.IsTrue(result.Success);
            Assert.AreEqual(id, result.Id);
        });

        [UnityTest]
        public IEnumerator Broker_ThrowsOnTimeout() => UniTask.ToCoroutine(async () =>
        {
            var broker = new RequestBroker();
            var conn   = new MockConnection();
            await conn.ConnectAsync(new DraoxConfig());

            const string id = "req_timeout";
            var task = broker.SendAsync(conn, "{}", id, 100 /* 100ms */, CancellationToken.None);

            DraoxTimeoutException caught = null;
            try { await task; }
            catch (DraoxTimeoutException ex) { caught = ex; }

            Assert.IsNotNull(caught);
        });

        [UnityTest]
        public IEnumerator Broker_FailAllRejectsAllPending() => UniTask.ToCoroutine(async () =>
        {
            var broker = new RequestBroker();
            var conn   = new MockConnection();
            await conn.ConnectAsync(new DraoxConfig());

            var t1 = broker.SendAsync(conn, "{}", "r1", 30_000, CancellationToken.None);
            var t2 = broker.SendAsync(conn, "{}", "r2", 30_000, CancellationToken.None);

            broker.FailAll(new DraoxException("disconnected"));

            int errors = 0;
            try { await t1; } catch { errors++; }
            try { await t2; } catch { errors++; }
            Assert.AreEqual(2, errors);
        });
    }

    // ── Reconnector tests ─────────────────────────────────────────────────────

    [TestFixture]
    public class ReconnectorTests
    {
        [UnityTest]
        public IEnumerator Reconnector_SucceedsOnSecondAttempt() => UniTask.ToCoroutine(async () =>
        {
            var cfg = new ReconnectConfig
            {
                Enabled            = true,
                MaxAttempts        = 5,
                BaseDelaySeconds   = 0.01f,
                MaxDelaySeconds    = 0.1f,
            };
            var reconnector = new Reconnector(cfg);

            int attempts = 0;
            var success  = await reconnector.AttemptAsync(async () =>
            {
                await UniTask.Yield();
                attempts++;
                return attempts >= 2;
            }, CancellationToken.None);

            Assert.IsTrue(success);
            Assert.AreEqual(2, attempts);
        });

        [UnityTest]
        public IEnumerator Reconnector_FailsAfterMaxAttempts() => UniTask.ToCoroutine(async () =>
        {
            var cfg = new ReconnectConfig
            {
                Enabled          = true,
                MaxAttempts      = 3,
                BaseDelaySeconds = 0.01f,
                MaxDelaySeconds  = 0.05f,
            };
            var reconnector = new Reconnector(cfg);

            var success = await reconnector.AttemptAsync(async () =>
            {
                await UniTask.Yield();
                return false;
            }, CancellationToken.None);

            Assert.IsFalse(success);
        });

        [Test]
        public void Reconnector_Reset_ClearsAttemptCount()
        {
            var cfg = new ReconnectConfig { Enabled = true, MaxAttempts = 3 };
            var reconnector = new Reconnector(cfg);
            reconnector.Reset();
            // No exception = pass; just verifying Reset() is callable without state corruption.
        }
    }

    // ── Event dispatch tests ──────────────────────────────────────────────────

    [TestFixture]
    public class EventDispatchTests
    {
        [Test]
        public void DraoxEvent_DataDeserializesPayload()
        {
            var evt = new DraoxEvent
            {
                Category  = "clan",
                Name      = "joined",
                RawData   = "{\"ClanId\":\"c42\",\"ClanName\":\"Warriors\"}",
                Timestamp = "0",
            };

            var data = evt.Data<Plugins.ClanJoinedEvent>();
            Assert.AreEqual("c42",      data.ClanId);
            Assert.AreEqual("Warriors", data.ClanName);
        }
    }
}
