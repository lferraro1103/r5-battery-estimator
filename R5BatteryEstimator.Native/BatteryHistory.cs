using System.Text.Json;

namespace R5BatteryEstimator.Native;

internal sealed record BatteryObservation(DateTimeOffset At, int Percent, bool Charging);

internal sealed class BatteryHistory
{
    private const int RetentionDays = 14;
    private readonly string _path;
    private readonly List<BatteryObservation> _items;

    public BatteryHistory()
    {
        var directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "R5 Battery Estimator");
        Directory.CreateDirectory(directory);
        _path = Path.Combine(directory, "battery-history.json");
        try { _items = JsonSerializer.Deserialize<List<BatteryObservation>>(File.ReadAllText(_path)) ?? []; }
        catch { _items = []; }
        Trim();
    }

    public IReadOnlyList<BatteryObservation> Recent(TimeSpan span) => _items.Where(item => item.At >= DateTimeOffset.Now - span).ToList();

    public void Add(int percent, bool charging)
    {
        _items.Add(new BatteryObservation(DateTimeOffset.Now, percent, charging));
        Trim();
        try { File.WriteAllText(_path, JsonSerializer.Serialize(_items)); }
        catch { /* Battery UI stays usable if local history cannot be saved. */ }
    }

    public double? LearnedFullChargeHours()
    {
        // A short drop is not evidence of a full battery cycle. Do not claim a
        // personalized full-charge duration until this installation has seen a
        // near-full state followed by a near-empty state while discharging.
        var fullStart = _items.FindIndex(item => item.Percent >= 95 && !item.Charging);
        if (fullStart < 0 || !_items.Skip(fullStart).Any(item => item.Percent <= 5 && !item.Charging)) return null;

        double elapsedHours = 0;
        double percentDropped = 0;
        for (var index = fullStart + 1; index < _items.Count; index++)
        {
            var before = _items[index - 1];
            var after = _items[index];
            var gap = after.At - before.At;
            if (before.Charging || after.Charging || gap <= TimeSpan.Zero || gap > TimeSpan.FromMinutes(10) || after.Percent > before.Percent) continue;
            elapsedHours += gap.TotalHours;
            percentDropped += before.Percent - after.Percent;
        }
        if (percentDropped < 3 || elapsedHours < 0.5) return null;
        return Math.Clamp(100 / (percentDropped / elapsedHours), 5, 1_000);
    }

    private void Trim()
    {
        var cutoff = DateTimeOffset.Now.AddDays(-RetentionDays);
        _items.RemoveAll(item => item.At < cutoff);
    }
}
