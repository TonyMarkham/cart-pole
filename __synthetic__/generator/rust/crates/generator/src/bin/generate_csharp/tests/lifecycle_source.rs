//! GenerationSet.Lifecycle.cs from protoc's GenerationSet.cs.

use crate::lifecycle::lifecycle_source;

/// The lines of protoc's C# that the lifecycle file is read from, as protoc
/// 3.36.2 writes them for generation_set.proto, with the members cut.
const GENERATED: &str = "\
namespace Simulation.CartPole.SyntheticDataGen.Proto {

  public static partial class GenerationSetReflection {
  }
  public sealed partial class Range : pb::IMessage<Range>
  #if !GOOGLE_PROTOBUF_REFSTRUCT_COMPATIBILITY_MODE
      , pb::IBufferMessage
  #endif
  {
    private static readonly pb::MessageParser<Range> _parser = new pb::MessageParser<Range>(() => new Range());
  }
  public sealed partial class GenerationSet : pb::IMessage<GenerationSet>
  {
  }
}
";

#[test]
fn given_protocs_classes_when_written_then_each_gets_the_attribute_with_its_modifiers() {
    // given
    let generated = GENERATED;

    // when
    let source = lifecycle_source(generated).unwrap();

    // then
    assert!(source.contains(
        "    [NoAutoStaticsCleanup]\n    public static partial class GenerationSetReflection { }"
    ));
    assert!(
        source.contains("    [NoAutoStaticsCleanup]\n    public sealed partial class Range { }")
    );
    assert!(
        source.contains(
            "    [NoAutoStaticsCleanup]\n    public sealed partial class GenerationSet { }"
        )
    );
    assert_eq!(source.matches("[NoAutoStaticsCleanup]").count(), 3);
}

#[test]
fn given_protocs_namespace_when_written_then_the_file_uses_it() {
    // given
    let generated = GENERATED;

    // when
    let source = lifecycle_source(generated).unwrap();

    // then
    assert!(source.contains("namespace Simulation.CartPole.SyntheticDataGen.Proto\n{\n"));
}

#[test]
fn given_any_output_when_written_then_it_is_all_inside_the_lifecycle_define() {
    // given
    let generated = GENERATED;

    // when
    let source = lifecycle_source(generated).unwrap();

    // then: the using too, so editors before 6000.5 compile nothing
    let opened = source.find("#if LIFECYCLE_APIS_AVAILABLE").unwrap();
    let using = source
        .find("using Unity.Scripting.LifecycleManagement;")
        .unwrap();
    assert!(opened < using);
    assert!(source.trim_end().ends_with("#endif"));
}

#[test]
fn given_no_namespace_when_written_then_it_is_an_error() {
    // given
    let generated = "public sealed partial class Range { }\n";

    // when
    let result = lifecycle_source(generated);

    // then
    assert!(result.is_err());
}

#[test]
fn given_no_classes_when_written_then_it_is_an_error() {
    // given
    let generated = "namespace Simulation.CartPole.SyntheticDataGen.Proto {\n}\n";

    // when
    let result = lifecycle_source(generated);

    // then
    assert!(result.is_err());
}
