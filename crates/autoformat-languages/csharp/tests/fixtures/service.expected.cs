using System;
using System.Collections.Generic;
namespace Demo.Services {
    #region Models
    [Serializable]
    public record Person(string Name, int Age);
    #endregion

    public interface IRepository<T> where T : class {
        T? Find(int id);
    }

    public class PersonService : IRepository<Person>, IDisposable {
        private readonly Dictionary<int, Person> _people = new();
        private int _count;  // number of lookups

        public PersonService() : base() {
        }

        public Person? Find(int id) {
            _count++;
            if (_people.TryGetValue(id, out var person)) {
                return person;
            } else {
                return null;
            }
        }

        public string Describe(Person p) => p switch
        {
            { Age: < 18 } => $"{p.Name} is young",
            _ => $"{p.Name,-10} is {p.Age}"
        };

        public void Dispose() {
#if DEBUG
            Console.WriteLine($"lookups: {_count}");
#endif
            var path = @"C:\temp\log.txt";
            var json = """
                { "a": 1 }
                """;
            var items = new List<int> { 1, 2, 3 };
            var first = items?.Count > 0 ? items[0] : -1;
            foreach (var item in items) { Console.WriteLine(item); }
            try { Risky(); } catch (Exception ex) when (ex is not null) { Log(ex); } finally { Cleanup(); }
            switch (first) {
                case 1:
                    break;
                default:
                    return;
            }
        }
    }
}
