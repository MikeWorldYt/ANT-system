import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { content } from '../../docs/content/content';
import { LanguageService } from '../../services/navLanguage.service';
import { Subscription } from 'rxjs';

@Component({
  selector: 'unordered-list',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './unordered-list.component.html',
  styleUrl: './unordered-list.component.css'
})
export class UnorderedListComponent {
  @Input() path!: [string, string, string];
  @Input() article!: string;
  @Input() write!: string;

  constructor(
    private languageService: LanguageService,
  ) { }

  items: string[] = [];

  ngOnInit(): void {
    this.article = this.concatArticle(this.article);
    this.subscribeToLanguageChanges();
    this.resolveContent();
  }

  languageSubscription!: Subscription;
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((Lang) => {
      this.path[0] = Lang;
      this.resolveContent();
    });
  }

  private resolveContent(): void {
    const [lang, title, page] = this.path;
    const article = this.article;
    const write = this.write;
    const listContent = content[lang][title][page][article][write];
    this.items = Object.values(listContent);
  }

  private concatArticle(article: string): string {
    return `article_${article}`;
  }

}
