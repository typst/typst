(function (r, c, g, h) {
  (r[g] = r[g] || []),
    r[g].push({ start: new Date().getTime(), event: "stg.start" });
  var u = c.getElementsByTagName("script")[0],
    n = c.createElement("script");
  function b(s, t, i) {
    var a = "";
    if (i) {
      var e = new Date();
      e.setTime(e.getTime() + 24 * i * 60 * 60 * 1e3),
        (a = "; expires=" + e.toUTCString()),
        (f = "; SameSite=Strict");
    }
    c.cookie = s + "=" + t + a + f + "; path=/";
  }
  var o =
    (r.location.href.match("stg_debug") || c.cookie.match("stg_debug")) &&
    !r.location.href.match("stg_disable_debug");
  b("stg_debug", o ? 1 : "", o ? 14 : -1);
  var p = [];
  g !== "dataLayer" && p.push("data_layer_name=" + g), o && p.push("stg_debug");
  var m = p.length > 0 ? "?" + p.join("&") : "";
  (n.async = !0),
    (n.src = "https://typst.containers.piwik.pro/" + h + ".js" + m),
    u.parentNode.insertBefore(n, u),
    (function (s, t, i) {
      s[t] = s[t] || {};
      for (var a = 0; a < i.length; a++)
        (function (e) {
          (s[t][e] = s[t][e] || {}),
            (s[t][e].api =
              s[t][e].api ||
              function () {
                var l = [].slice.call(arguments, 0);
                typeof l[0] == "string" &&
                  r[g].push({
                    event: t + "." + e + ":" + l[0],
                    parameters: [].slice.call(arguments, 1),
                  });
              });
        })(i[a]);
    })(r, "ppms", ["tm", "cm"]);
})(window, document, "dataLayer", "62943705-1a31-4e24-bb2a-c9111d7b1f32");
