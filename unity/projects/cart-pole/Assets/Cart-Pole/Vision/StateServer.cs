using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using Simulation.CartPole.SyntheticDataGen;
using UnityEngine;

namespace CartPole.Vision
{
    // Unity 6000.7.0b2's UAL analyzer demands AutoStaticsCleanup/NoAutoStatics-
    // Cleanup attributes that its own assemblies do not ship yet; the static
    // cleanup is done manually instead, which behaves identically for the
    // request queue.
#pragma warning disable UAL0010, UAL0013
    // TCP bridge between the gym (Python) and the rendered assembly.
    //
    // Protocol (localhost): the client sends one JSON object per line:
    //   {"id": 7, "joints": [{"name": "02-Cart.Slider.joint", "qpos": 0.05},
    //                         {"name": "04-Pole.Hinge.joint", "qpos": 0.3}]}
    // The main thread applies the state through the JointDriver, renders the
    // assigned camera, and answers with a 4-byte little-endian length header
    // followed by JPEG bytes.
    // AutoStaticsCleanup: the static request queue holds TCP clients that
    // are dead after a domain reload - Unity must clear it when entering
    // Play mode so stale requests never resurface.
    public sealed class StateServer : MonoBehaviour
    {
        public int port = 5555;
        public JointDriver driver;
        public MarkerManager markerManager;
        public Camera renderCamera;
        public int captureWidth = 960;
        public int captureHeight = 540;

        private sealed class Request
        {
            public TcpClient client;
            public string json;
        }

        [Serializable]
        private sealed class StateMessage
        {
            public int id = 0;
            public JointCmd[] joints = new JointCmd[0];

            // Optional rig state: when apply is set, the cart and pole
            // are positioned through the MarkerManager's rig before the
            // capture - {"rig": {"apply": true, "cartPosition": 0.1,
            // "poleRotation": 0.3}}.
            public RigStateCmd rig;
        }

        [Serializable]
        private sealed class RigStateCmd
        {
            public bool apply;
            public float cartPosition;
            public float poleRotation;
        }

        [Serializable]
        private sealed class JointCmd
        {
            public string name = "";
            public float qpos = 0f;
        }

        private static readonly ConcurrentQueue<Request> Inbox =
            new ConcurrentQueue<Request>();

        private TcpListener listener;
        private Thread acceptThread;
        private RenderTexture target;
        private Texture2D readback;

        #if UNITY_EDITOR
        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.SubsystemRegistration)]
        private static void ResetStaticInbox()
        {
            while (Inbox.TryDequeue(out _))
            {
            }
        }
        #endif

        private void Start()
        {
            target = new RenderTexture(captureWidth, captureHeight, 24);
            readback = new Texture2D(captureWidth, captureHeight, TextureFormat.RGB24, false);
            if (driver != null)
            {
                driver.CaptureBasePose();
            }
            listener = new TcpListener(IPAddress.Loopback, port);
            listener.Start();
            acceptThread = new Thread(AcceptLoop) { IsBackground = true };
            acceptThread.Start();
            Debug.Log($"[StateServer] listening on 127.0.0.1:{port}");
        }

        private void AcceptLoop()
        {
            try
            {
                while (true)
                {
                    var client = listener.AcceptTcpClient();
                    var thread = new Thread(() => ReadLoop(client)) { IsBackground = true };
                    thread.Start();
                }
            }
            catch (Exception)
            {
                // listener stopped with the component
            }
        }

        private static void ReadLoop(TcpClient client)
        {
            try
            {
                using var reader = new StreamReader(client.GetStream(), Encoding.UTF8);
                string line;
                while ((line = reader.ReadLine()) != null)
                {
                    if (line.Length == 0)
                    {
                        continue;
                    }
                    // Unity APIs are main-thread only: stash for Update()
                    Inbox.Enqueue(new Request { client = client, json = line });
                }
            }
            catch (Exception)
            {
                // client disconnected
            }
        }

        private void Update()
        {
            while (Inbox.TryDequeue(out var request))
            {
                Handle(request);
            }
        }

        private void Handle(Request request)
        {
            try
            {
                var message = JsonUtility.FromJson<StateMessage>(request.json);
                var qpos = new Dictionary<string, float>();
                if (message.joints != null)
                {
                    foreach (var joint in message.joints)
                    {
                        qpos[joint.name] = joint.qpos;
                    }
                }
                if (message.rig != null && message.rig.apply &&
                    markerManager != null)
                {
                    markerManager.ApplyState(message.rig.cartPosition, message.rig.poleRotation);
                }
                if (driver != null)
                {
                    driver.Apply(qpos);
                }
                var jpeg = CaptureJpeg();
                var stream = request.client.GetStream();
                var header = BitConverter.GetBytes(jpeg.Length);
                stream.Write(header, 0, header.Length);
                stream.Write(jpeg, 0, jpeg.Length);
                stream.Flush();
            }
            catch (Exception exception)
            {
                Debug.LogWarning($"[StateServer] request failed: {exception.Message}");
            }
        }

        private byte[] CaptureJpeg()
        {
            var previous = renderCamera.targetTexture;
            renderCamera.targetTexture = target;
            renderCamera.Render();
            renderCamera.targetTexture = previous;
            RenderTexture.active = target;
            readback.ReadPixels(new Rect(0, 0, captureWidth, captureHeight), 0, 0);
            readback.Apply();
            RenderTexture.active = null;
            return readback.EncodeToJPG(90);
        }

        private void OnDestroy()
        {
            try
            {
                listener?.Stop();
            }
            catch (Exception)
            {
            }
        }
    }
}
