namespace Simulation.CartPole.SimLoop
{
    // What one scene does when it runs: the same scene and build serve both,
    // and PlayerController turns on the component of the chosen one.
    public enum PlayerMode
    {
        // The synthetic loop's camera: SimulationLoopController renders each
        // joint state the StateServer applies and replies with its JPEG.
        SyntheticCamera,

        // The dataset generator: SyntheticDataGenerator draws states and
        // writes labeled frames.
        SyntheticGenerator,
    }
}
