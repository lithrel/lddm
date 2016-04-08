require 'rubygems'
require 'optparse'
require 'nokogiri'
require 'open-uri'
require 'json'

options = {}
OptionParser.new do |opts|
  opts.banner = "Usage: scraper.rb [options]"
  opts.on('-e', '--episode NUMERO', 'Episode') { |e| options[:episode] = e }
  opts.on('-o', '--overwrite-cache', 'Overwrite') { |o| options[:overwrite] = o }
end.parse!

filepath = 'cache/episode_' + options[:episode] + '.html'
url = "http://www.radiokawa.com/episode/les-demons-du-midi-" + options[:episode] + "/"

if File.exist?(filepath) && !options[:overwrite]
    doc = Nokogiri::HTML(open(filepath))
else
    doc = Nokogiri::HTML(open(url))
    File.write(filepath, doc.to_html)
end

playlist = doc.at_css(".spoiler").text.gsub(/[\n]+/, "\n")
lines = playlist.split("\n")
tracks = []

lines.each_with_index{ |line, index|
    line = line.gsub(/\s|\u00A0/, ' ')
    line.strip!
    if line.to_s == ''
        next
    elsif m = line.match("^([0-9]{1,2}) ?- (.*) - ([^,]+), (.*)")
        tracks.push({
            "id" => options[:episode] + '-' + index.to_s,
            "episode_id" => options[:episode].to_i,
            "num_track" => m[1].to_i,
            "num_played" => index.to_i,
            "title" => m[3],
            "game_title" => m[2],
            "author" => m[4],
            "author_link" => '',
            "type" => 'tracklist',
        })
    elsif m = line.match("^([0-9]{1,2}) ?- (.*) - (.*)")
        tracks.push({
            "id" => options[:episode] + '-' + index.to_s,
            "episode_id" => options[:episode].to_i,
            "num_track" => m[1].to_i,
            "num_played" => index.to_i,
            "title" => m[2],
            "game_title" => m[2],
            "author" => m[3],
            "author_link" => '',
            "type" => 'tracklist',
        })
    elsif m = line.match("^([0-9]{1,2}) ?- ([^,]+), (.*)")
        tracks.push({
            "id" => options[:episode] + '-' + index.to_s,
            "episode_id" => options[:episode].to_i,
            "num_track" => m[1].to_i,
            "num_played" => index.to_i,
            "title" => m[2],
            "game_title" => m[2],
            "author" => m[3],
            "author_link" => '',
            "type" => 'tracklist',
        })
    elsif m = line.match("^Actu ([0-9]{1}) : (.*) - ([^,]+), (.*)")
        tracks.push({
            "id" => options[:episode] + '-' + index.to_s,
            "episode_id" => options[:episode],
            "num_track" => '',
            "num_played" => index,
            "title" => m[3],
            "game_title" => m[2],
            "author" => m[4],
            "type" => 'actu',
        })
    elsif m = line.match('^Reprise du mois : ([^,]+), (.*)')
        tracks.push({
            "id" => options[:episode] + '-' + index.to_s,
            "episode_id" => options[:episode],
            "num_track" => '',
            "num_played" => index,
            "title" => m[1],
            "game_title" => m[2],
            "author" => '',
            "type" => 'reprise',
        })
    elsif m = line.match("^L'interlude de l'invité : ([^(]+) \(.*\) vous propose (.*)")
        tracks.push({
            "id" => options[:episode] + '-' + index.to_s,
            "episode_id" => options[:episode],
            "num_track" => '',
            "num_played" => index,
            "title" => m[3],
            "game_title" => '',
            "author" => '',
            "author_link" => '',
            "guest" => m[1],
            "type" => 'interlude',
        })
    else
        puts 'NO MATCH :: ' + line + ' ::'
    end
}

episode = {
    "id" => options[:episode].to_i,
    "title" => doc.at_css(".show-subtitle").text,
    "date" => DateTime.rfc3339(doc.at_css(".pubdate time").attr('datetime')).strftime('%d/%m/%Y'),
    "url" => url,
    "duration" => '',
    "tags" => [],
    "track_count" => tracks.count
}

puts JSON.pretty_generate(JSON.parse(episode.to_json))
puts JSON.pretty_generate(JSON.parse(tracks.to_json))